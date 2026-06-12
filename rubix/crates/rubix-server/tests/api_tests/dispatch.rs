//! Inbound spark dispatch: a spark published on the bus activates an embedded
//! agent run (a job, not a chat). A scripted executor drives the agent offline;
//! the run commands a point, and observing that command proves the published
//! finding reached the agent and ran end-to-end.

use std::sync::Arc;
use std::time::Duration;

use awaken_runtime::engine::{ProviderScriptEvent, ScriptedLlmExecutor};
use rubix_server::agent::build_runtime_with_executor;
use rubix_server::bus::ZenohBus;
use rubix_server::dispatch::Dispatcher;
use rubix_server::store::Store;
use rubix_server::AppState;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn published_spark_activates_an_agent_run() {
    // A store + bus the dispatcher and HTTP API share.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("dispatch.db")).expect("store");
    let bus = ZenohBus::open(store.clone()).await.expect("bus");
    bus.serve().await.expect("serve");

    // Build an agent whose single scripted turn commands a fan at priority 14
    // (at/below the agent ceiling 13 → allowed, completes without suspending).
    let state = AppState {
        store: store.clone(),
        bus: Some(bus.clone()),
        query: None,
        agent: None,
        ai_min_priority: 13,
        ai_escalation_floor: 1,
    };
    let script = [ProviderScriptEvent::ToolCall {
        id: "c1".into(),
        name: "rubix_write_point".into(),
        arguments: json!({
            "point": "disp/s1/ahu-3/fan", "value": true, "priority": 14
        }),
        tokens: Default::default(),
    }];
    let executor = Arc::new(ScriptedLlmExecutor::new(script));
    let runtime = Arc::new(
        build_runtime_with_executor(&state, "scripted", "test-model", "test-model", 4, executor)
            .expect("runtime"),
    );

    // Provision the site/equip/point the agent will command, over the same store.
    let app = TestApp::with_store_at(store.clone());
    let site = app.create_site_with("disp", "s1").await;
    let equip = app.create_equip(&site).await;
    let fan = app.create_point(&equip, "cmd", "fan").await;

    // Launch dispatch, then publish a finding as a second peer would.
    let dispatcher = Dispatcher::launch(bus.clone(), runtime, store.clone());
    tokio::time::sleep(Duration::from_millis(500)).await; // let the subscriber attach

    let spark = json!({
        "id": "00000000-0000-0000-0000-0000000000aa",
        "site_id": site,
        "rule": "heat_cool_conflict",
        "severity": "fault",
        "message": "AHU-3 heating and cooling at once",
        "point_ids": [],
        "ts": "2026-06-12T00:00:00Z",
        "acknowledged": false
    });
    bus.session_clone()
        .put(
            "disp/s1/spark/heat_cool_conflict/aa",
            serde_json::to_vec(&spark).unwrap(),
        )
        .await
        .expect("publish spark");

    // The dispatched run should command the fan within a couple of seconds.
    let mut commanded = false;
    for _ in 0..30 {
        let (_, fan_body) = app
            .request("GET", &format!("/api/v1/points/{fan}"), None)
            .await;
        if fan_body["point"]["cur_value"] == json!(true) {
            commanded = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    dispatcher.shutdown().await;
    assert!(commanded, "published spark did not activate an agent run that commanded the point");
}
