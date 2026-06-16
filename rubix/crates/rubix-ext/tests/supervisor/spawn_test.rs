//! Integration: the supervisor turns a process spec into a live, supervised
//! child and reports its runtime state — the heart of the extension runtime
//! (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase 1).
//!
//! These spawn a *real* child process (the `echo_child` example, which speaks the
//! supervisor's `Content-Length` JSON-RPC handshake) so the spawn → init
//! handshake → health-ping → shutdown path is exercised end to end, not mocked.
//!
//! Four halves of one contract:
//!
//! - **A healthy child comes up.** After the init handshake the supervisor
//!   reports `Running`, a live pid, sampled process stats, and `is_live()`.
//! - **The child runs under its principal.** The supervisor injects the
//!   extension's identity env, so the child observes its own subject.
//! - **A handshake that never reports ready never reaches `Running`** and, under
//!   a `Never` policy, settles stopped/failed rather than being treated as up.
//! - **A crash restarts** under an `Always` policy — the restart counter climbs.

use std::path::PathBuf;
use std::time::Duration;

use rubix_ext::supervisor::{
    Backoff, ExtensionId, Identity, LifecycleState, ProcessSpec, RestartPolicy, Supervisor,
};

/// The compiled `echo_child` helper bin. Cargo sets `CARGO_BIN_EXE_<name>` for a
/// package's integration tests, so this resolves to the built binary regardless
/// of profile or workspace layout — and `cargo test` always builds the package's
/// bins (unlike examples), so the helper is guaranteed present.
fn echo_child_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_echo_child"))
}

/// A spec pointing at the example child with fast supervision timings so the
/// tests do not idle on the default multi-second cadences.
fn fast_spec(restart: RestartPolicy) -> ProcessSpec {
    let mut spec = ProcessSpec::new(echo_child_bin());
    spec.restart = restart;
    spec.max_restarts = 3;
    spec.within_seconds = 60;
    spec.backoff = Backoff {
        initial_ms: 5,
        max_ms: 20,
        jitter: false,
    };
    spec.health.interval_ms = 100;
    spec.health.timeout_ms = 200;
    spec.shutdown_grace_ms = 500;
    spec
}

fn identity() -> Identity {
    Identity {
        namespace: "rubix".to_owned(),
        subject: "echo-ext".to_owned(),
        secret: "k".to_owned(),
    }
}

fn id() -> ExtensionId {
    ExtensionId::new("rubix", "echo-ext")
}

/// Await a watch-channel reaching `want`, or panic after `timeout`.
async fn wait_for_state(
    handle: &rubix_ext::supervisor::SupervisorHandle,
    want: LifecycleState,
    timeout: Duration,
) {
    let mut rx = handle.state();
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if *rx.borrow() == want {
            return;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!("timed out waiting for {want:?}, last was {:?}", *rx.borrow());
        }
        if tokio::time::timeout(remaining, rx.changed()).await.is_err() {
            panic!("timed out waiting for {want:?}, last was {:?}", *rx.borrow());
        }
    }
}

#[tokio::test]
async fn a_healthy_child_comes_up_running_with_a_pid() {
    assert!(
        echo_child_bin().exists(),
        "echo_child example must be built (cargo test builds examples): {:?}",
        echo_child_bin()
    );
    let handle = Supervisor::start(id(), fast_spec(RestartPolicy::OnCrash), identity())
        .expect("start supervisor");

    wait_for_state(&handle, LifecycleState::Running, Duration::from_secs(5)).await;
    assert!(handle.is_live(), "a Running child is live");
    assert!(handle.pid().is_some(), "a Running child has a pid");
    let stats = handle.process_stats().expect("Running child has stats");
    assert_eq!(stats.pid, handle.pid().unwrap());

    // Let a couple of health ticks land so the wire loop is genuinely serving.
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert!(handle.is_live(), "child still live after health pings");

    handle.shutdown().await;
    wait_for_state(&handle, LifecycleState::Stopped, Duration::from_secs(5)).await;
    assert!(!handle.is_live(), "a stopped child is not live");
    assert!(handle.pid().is_none(), "no pid once stopped");
}

#[tokio::test]
async fn a_handshake_that_never_reports_ready_never_runs() {
    let mut spec = fast_spec(RestartPolicy::Never);
    spec.env.push(("RUBIX_CHILD_NO_READY".to_owned(), "1".to_owned()));
    let handle = Supervisor::start(id(), spec, identity()).expect("start supervisor");

    // Under a Never policy a failed handshake settles Stopped without ever
    // flipping to Running.
    wait_for_state(&handle, LifecycleState::Stopped, Duration::from_secs(5)).await;
    assert!(!handle.is_live());
    assert!(!handle.lifecycle_state().is_running());
}

#[tokio::test]
async fn a_crashing_child_is_restarted_under_an_always_policy() {
    let mut spec = fast_spec(RestartPolicy::Always);
    // Crash right after the init handshake completes, every time.
    spec.env.push(("RUBIX_CHILD_CRASH".to_owned(), "1".to_owned()));
    let handle = Supervisor::start(id(), spec, identity()).expect("start supervisor");

    // It crashes repeatedly until the intensity cap (3) trips, settling Failed.
    wait_for_state(&handle, LifecycleState::Failed, Duration::from_secs(5)).await;
    assert!(
        handle.restarts_total() >= 1,
        "a crashing child was restarted at least once before failing"
    );
    assert!(handle.lifecycle_state().is_terminal());
}
