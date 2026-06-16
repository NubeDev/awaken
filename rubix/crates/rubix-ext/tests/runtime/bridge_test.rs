//! Integration: the bridge turns the gated lifecycle record into a running
//! process, and the boot reconciler rebuilds the supervisor map from those
//! records (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase 2).
//!
//! Two halves of one contract — the gated record is the single source of truth,
//! and both the live handler and a cold boot drive the supervisor *from* it:
//!
//! - **Handler-drives.** A granted `start` crosses the gate, spawns a child, and
//!   reports a correlation id; a later `stop` tears it down. An out-of-grant
//!   `start` is denied before any process is touched.
//! - **Boot reconciler.** Given the persisted control records, `start` records
//!   are brought up and `stop` records stay down — idempotently.

#[path = "../ext/mod.rs"]
mod ext;

use std::path::PathBuf;
use std::time::Duration;

use rubix_core::{Id, Record};
use rubix_gate::{Capability, create_grant};

use rubix_ext::supervisor::{
    Backoff, ExtensionId, Identity, ProcessSpec, RestartPolicy,
};
use rubix_ext::{ControlMethod, ControlRequest, ExtError};
use rubix_ext::runtime::{ExtensionRuntime, drive_lifecycle, reconcile_from_records};

use ext::open::{admin, open_ext_store};

const TENANT: &str = "rubix";

fn echo_child_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_echo_child"))
}

fn fast_spec() -> ProcessSpec {
    let mut spec = ProcessSpec::new(echo_child_bin());
    spec.restart = RestartPolicy::OnCrash;
    spec.backoff = Backoff {
        initial_ms: 5,
        max_ms: 20,
        jitter: false,
    };
    spec.health.interval_ms = 100;
    spec.health.timeout_ms = 200;
    spec.shutdown_grace_ms = 300;
    spec
}

fn lifecycle_request(action: &str) -> ControlRequest {
    ControlRequest::new(
        ControlMethod::Lifecycle,
        Id::from_raw("ext-control-rec"),
        serde_json::json!({ "action": action }),
    )
}

async fn wait_running(handle: &rubix_ext::supervisor::SupervisorHandle, timeout: Duration) {
    let mut rx = handle.state();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if rx.borrow().is_running() {
            return;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() || tokio::time::timeout(remaining, rx.changed()).await.is_err() {
            panic!("timed out waiting for Running, last {:?}", *rx.borrow());
        }
    }
}

#[tokio::test]
async fn a_granted_start_spawns_a_child_and_stop_tears_it_down() {
    let handle = open_ext_store("rt_bridge_start").await;
    let registration = rubix_ext::register_extension(handle.raw(), "drive-ext", TENANT, "k")
        .await
        .expect("register extension");
    let extension = registration.principal().clone();
    create_grant(handle.raw(), &admin(), &extension, Capability::ExtensionManage)
        .await
        .expect("grant extension-manage");

    let rt = ExtensionRuntime::new();
    let identity = Identity {
        namespace: TENANT.to_owned(),
        subject: "drive-ext".to_owned(),
        secret: "k".to_owned(),
    };
    let id = ExtensionId::from(&extension);

    // ---- start ----
    let outcome = drive_lifecycle(
        &rt,
        handle.raw(),
        &extension,
        &id,
        &lifecycle_request("start"),
        fast_spec(),
        identity.clone(),
    )
    .await
    .expect("granted start drives the supervisor");
    assert!(!outcome.correlation_id.as_str().is_empty(), "gate stamped a correlation id");
    assert!(outcome.state.is_some(), "start reports a supervisor state");

    let sup = rt.supervisors.get(&id).expect("supervisor registered after start");
    wait_running(&sup, Duration::from_secs(5)).await;
    assert!(sup.is_live());

    // The command was counted on the metrics registry.
    assert_eq!(rt.metrics.snapshot(&id).commands, 1);

    // ---- stop ----
    drive_lifecycle(
        &rt,
        handle.raw(),
        &extension,
        &id,
        &lifecycle_request("stop"),
        fast_spec(),
        identity,
    )
    .await
    .expect("granted stop drives the supervisor");
    assert!(rt.supervisors.get(&id).is_none(), "supervisor torn down on stop");
    assert_eq!(rt.metrics.snapshot(&id).commands, 2);
}

#[tokio::test]
async fn an_out_of_grant_start_spawns_nothing() {
    let handle = open_ext_store("rt_bridge_deny").await;
    // No extension-manage grant conferred.
    let extension = rubix_ext::register_extension(handle.raw(), "nogrant-ext", TENANT, "k")
        .await
        .expect("register extension")
        .principal()
        .clone();

    let rt = ExtensionRuntime::new();
    let id = ExtensionId::from(&extension);
    let identity = Identity {
        namespace: TENANT.to_owned(),
        subject: "nogrant-ext".to_owned(),
        secret: "k".to_owned(),
    };

    let err = drive_lifecycle(
        &rt,
        handle.raw(),
        &extension,
        &id,
        &lifecycle_request("start"),
        fast_spec(),
        identity,
    )
    .await
    .expect_err("an out-of-grant start must be denied");
    assert!(matches!(err, ExtError::Denied(_)));
    assert!(rt.supervisors.get(&id).is_none(), "nothing spawned on a denial");
    assert_eq!(rt.metrics.snapshot(&id).command_errors, 1, "denial counted as a failed command");
}

#[tokio::test]
async fn the_reconciler_brings_start_records_up_and_skips_the_rest() {
    let rt = ExtensionRuntime::new();
    let records = vec![
        // A start record with a process runtime spec → should be brought up.
        Record::new(
            TENANT,
            serde_json::json!({
                "extension": "alpha",
                "lifecycle": "start",
                "runtime": { "bin": echo_child_bin(), "health": { "interval_ms": 100, "timeout_ms": 200 } },
            }),
        ),
        // A stop record → stays down.
        Record::new(
            TENANT,
            serde_json::json!({ "extension": "beta", "lifecycle": "stop", "runtime": { "bin": echo_child_bin() } }),
        ),
        // A start record with no runtime spec → skipped.
        Record::new(
            TENANT,
            serde_json::json!({ "extension": "gamma", "lifecycle": "start" }),
        ),
        // Not an extension control record (no lifecycle) → ignored entirely.
        Record::new(TENANT, serde_json::json!({ "temp": 21.5 })),
    ];

    let resolver = |_id: &ExtensionId| {
        Some(Identity {
            namespace: TENANT.to_owned(),
            subject: "x".to_owned(),
            secret: "k".to_owned(),
        })
    };

    let report = reconcile_from_records(&rt, &records, resolver).await;

    let alpha = ExtensionId::new(TENANT, "alpha");
    assert_eq!(report.started, vec![alpha.clone()]);
    assert!(report.stopped.is_empty(), "beta had no live supervisor to stop");
    // gamma (no runtime) is the one reasoned skip; the non-extension record is
    // dropped silently, not counted as a skip.
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].0, ExtensionId::new(TENANT, "gamma"));

    let sup = rt.supervisors.get(&alpha).expect("alpha supervised");
    wait_running(&sup, Duration::from_secs(5)).await;

    // Idempotent: a second reconcile does not respawn the live alpha.
    let again = reconcile_from_records(&rt, &records, resolver).await;
    assert_eq!(again.started, vec![alpha.clone()], "start is idempotent on a live child");
    assert!(rt.supervisors.get(&alpha).unwrap().is_live());

    rt.supervisors.stop(&alpha).await;
}
