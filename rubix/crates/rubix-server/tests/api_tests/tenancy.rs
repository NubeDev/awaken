//! Tenant confinement: a run carries the `{org}/{site}` it acts within, mapped
//! onto awaken's tenant `ScopeId`, and its tools refuse a point outside that
//! scope at the tool boundary — not merely at HTTP. These tests prove an agent
//! scoped to site A cannot read/command site B through its tools on either entry
//! path: the dispatch path (a spark names its site, the run inherits it) and the
//! chat-equivalent path (the principal's site scopes the run). A scripted
//! executor drives the agent offline; the cross-tenant point staying untouched
//! is the proof the tool call was denied. An in-scope control on each path shows
//! the confinement does not also block a legitimate same-tenant write.

use std::sync::Arc;
use std::time::Duration;

use awaken_runtime::engine::{ProviderScriptEvent, ScriptedLlmExecutor};
use awaken_runtime::run::RunActivation;
use awaken_runtime_contract::contract::inference::StopReason;
use awaken_runtime_contract::contract::message::Message;
use rubix_server::agent::{
    build_scoped_runtime, run_and_persist, runtime_for_scope, RunOrigin, RunStatus, RuntimeBlueprint,
    AGENT_ID,
};
use rubix_server::bus::ZenohBus;
use rubix_server::dispatch::Dispatcher;
use rubix_server::profile::{Profile, ProfileKind};
use rubix_server::store::Store;
use rubix_server::AppState;
use rubix_tools::TenantScope;
use serde_json::{json, Value};

use super::harness::TestApp;

/// A scripted turn that commands the named fan at priority 14 (at/below the
/// agent ceiling, so it does not suspend), followed by a closing chat response
/// so the agent loop ends cleanly rather than exhausting the script.
fn command_then_end(point: &str) -> [ProviderScriptEvent; 2] {
    [
        ProviderScriptEvent::ToolCall {
            id: "c1".into(),
            name: "rubix_write_point".into(),
            arguments: json!({ "point": point, "value": true, "priority": 14 }),
            tokens: Default::default(),
        },
        ProviderScriptEvent::ChatResponse {
            content: "done".into(),
            tokens: Default::default(),
            finish_reason: StopReason::EndTurn,
        },
    ]
}

/// Build an `AppState` over `store` with a scripted blueprint and an unscoped
/// boot runtime (the dispatcher rebuilds a scoped runtime per spark). The script
/// is whatever single-shot tool program the test wants the agent to attempt.
fn scripted_state(
    store: Store,
    bus: Option<ZenohBus>,
    script: impl IntoIterator<Item = ProviderScriptEvent>,
) -> (AppState, RuntimeBlueprint) {
    let executor = Arc::new(ScriptedLlmExecutor::new(script));
    let blueprint =
        RuntimeBlueprint::with_executor("scripted", "test-model", "test-model", 4, executor);
    let mut state = AppState {
        profile: Profile::defaults(ProfileKind::Edge),
        store,
        bus,
        query: None,
        his_tier: None,
        agent: None,
        agent_blueprint: Some(blueprint.clone()),
        ai_min_priority: 13,
        ai_escalation_floor: 1,
        authenticator: None,
    };
    state.agent = Some(Arc::new(
        build_scoped_runtime(&state, &blueprint, None).expect("boot runtime"),
    ));
    (state, blueprint)
}

/// The fan keyexpr for a tenant the harness provisions under it.
fn fan_keyexpr(org: &str, site: &str) -> String {
    format!("{org}/{site}/ahu-3/fan")
}

/// Publish a finding for `site_id` under the tenant's keyexpr prefix, as a peer
/// rule engine would, so the dispatcher activates a scoped run for it.
async fn publish_spark(bus: &ZenohBus, site_id: &str, prefix: &str, suffix: &str) {
    let spark = json!({
        "id": format!("00000000-0000-0000-0000-0000000000{suffix}"),
        "site_id": site_id,
        "rule": "heat_cool_conflict",
        "severity": "fault",
        "message": "tenancy probe",
        "point_ids": [],
        "ts": "2026-06-12T00:00:00Z",
        "acknowledged": false
    });
    bus.session_clone()
        .put(
            format!("{prefix}/spark/heat_cool_conflict/{suffix}"),
            serde_json::to_vec(&spark).unwrap(),
        )
        .await
        .expect("publish spark");
}

/// Read a point's current value over HTTP.
async fn cur_value(app: &TestApp, point_id: &str) -> Value {
    let (_, body) = app
        .request("GET", &format!("/api/v1/points/{point_id}"), None)
        .await;
    body["point"]["cur_value"].clone()
}

/// A dispatched run is confined to the spark's site: a spark for site A drives an
/// agent whose scripted turn commands a point in site B; the tool denies it, so
/// B's point is never commanded. A second spark — for site B — proves the same
/// agent program does land when the run is scoped to B (the denial is the scope,
/// not the script).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatched_run_cannot_command_a_point_in_another_tenant() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("tenancy-dispatch.db")).expect("store");
    let bus = ZenohBus::open(store.clone()).await.expect("bus");
    bus.serve().await.expect("serve");

    // Two tenants on the shared store; each has its own fan.
    let app = TestApp::with_store_at(store.clone());
    let site_a = app.create_site_with("ten", "sa").await;
    let equip_a = app.create_equip(&site_a).await;
    let fan_a = app.create_point(&equip_a, "cmd", "fan").await;
    let site_b = app.create_site_with("ten", "sb").await;
    let equip_b = app.create_equip(&site_b).await;
    let fan_b = app.create_point(&equip_b, "cmd", "fan").await;

    // Both runs replay the same program: command site B's fan, then end. The
    // sparks are dispatched one at a time (the shared scripted executor is a FIFO
    // queue), so the two `command_then_end` programs are consumed in order.
    let script = command_then_end(&fan_keyexpr("ten", "sb"))
        .into_iter()
        .chain(command_then_end(&fan_keyexpr("ten", "sb")));
    let (state, _) = scripted_state(store.clone(), Some(bus.clone()), script);

    let dispatcher = Dispatcher::launch(bus.clone(), state.clone());
    tokio::time::sleep(Duration::from_millis(500)).await; // let the subscriber attach

    // A spark for site A: the run is confined to `ten/sa`, so its attempt to
    // command `ten/sb/...` is refused at the tool boundary. Wait it out, then
    // assert B was never touched.
    publish_spark(&bus, &site_a, "ten/sa", "a1").await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(
        cur_value(&app, &fan_b).await.is_null(),
        "site-A run commanded site B's point — tenant scope not enforced at the tool"
    );

    // A spark for site B: the identical program is now in scope and lands.
    publish_spark(&bus, &site_b, "ten/sb", "b1").await;
    let mut commanded = false;
    for _ in 0..30 {
        if cur_value(&app, &fan_b).await == json!(true) {
            commanded = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    dispatcher.shutdown().await;
    assert!(commanded, "site-B run did not command site B's own point (scope blocked a legal write)");
    // Site A's fan was never the target; it must remain untouched throughout.
    assert!(cur_value(&app, &fan_a).await.is_null(), "site A's point was unexpectedly commanded");
}

/// The chat path scopes a run to the principal's `{org}/{site}`. Building the run
/// the way the chat endpoint does — a runtime scoped to site A — an agent turn
/// that commands a point in site B is refused: B's point stays untouched. The
/// same runtime commanding A's own point lands, so the confinement is the tenant
/// boundary, not a broken tool.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chat_scoped_run_cannot_command_a_point_in_another_tenant() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("tenancy-chat.db")).expect("store");

    let app = TestApp::with_store_at(store.clone());
    let site_a = app.create_site_with("ten", "ca").await;
    let equip_a = app.create_equip(&site_a).await;
    let fan_a = app.create_point(&equip_a, "cmd", "fan").await;
    let site_b = app.create_site_with("ten", "cb").await;
    let equip_b = app.create_equip(&site_b).await;
    let fan_b = app.create_point(&equip_b, "cmd", "fan").await;

    let scope_a = TenantScope::new("ten", "ca");

    // Cross-tenant: a scope-A runtime whose turn commands site B is denied.
    {
        let (state, _) =
            scripted_state(store.clone(), None, command_then_end(&fan_keyexpr("ten", "cb")));
        let runtime = runtime_for_scope(&state, Some(scope_a.clone()))
            .expect("scoped runtime")
            .expect("agent enabled");
        let activation = RunActivation::new(
            "t-cross".to_string(),
            vec![Message::user("force site B's fan on")],
        )
        .with_agent_id(AGENT_ID);
        let record = run_and_persist(&runtime, &store, RunOrigin::Chat, activation)
            .await
            .expect("run");
        // The run completes (the tool error is fed back to the loop, not a panic),
        // and the cross-tenant point was never commanded.
        assert_eq!(record.status, RunStatus::Completed, "{record:?}");
        assert!(
            cur_value(&app, &fan_b).await.is_null(),
            "scope-A chat run commanded site B's point — tenant scope not enforced at the tool"
        );
    }

    // In-scope control: the same scope-A runtime commanding A's own point lands.
    {
        let (state, _) =
            scripted_state(store.clone(), None, command_then_end(&fan_keyexpr("ten", "ca")));
        let runtime = runtime_for_scope(&state, Some(scope_a))
            .expect("scoped runtime")
            .expect("agent enabled");
        let activation = RunActivation::new(
            "t-in-scope".to_string(),
            vec![Message::user("turn my fan on")],
        )
        .with_agent_id(AGENT_ID);
        run_and_persist(&runtime, &store, RunOrigin::Chat, activation)
            .await
            .expect("run");
        assert_eq!(
            cur_value(&app, &fan_a).await,
            json!(true),
            "scope-A run did not command its own tenant's point (scope blocked a legal write)"
        );
    }
}
