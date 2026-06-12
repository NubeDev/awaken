//! `agent_call` board node: a board step raises an embedded-agent run. A
//! scripted executor drives the agent offline; the run commands a point, and
//! observing that command proves the board node activated the agent. Also
//! covers the recursion guard — a board access without an agent fails the call
//! closed (which is what the agent's own `run_board` tool gets).

use std::sync::Arc;
use std::time::Duration;

use awaken_runtime::engine::{ProviderScriptEvent, ScriptedLlmExecutor};
use awaken_runtime_contract::contract::inference::StopReason;
use rubix_flow::{AgentRequest, PointAccess, COMPONENTS};
use rubix_server::agent::build_runtime_with_executor;
use rubix_server::flow::StorePointAccess;
use rubix_server::store::Store;
use rubix_server::{app, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn req(router: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(j) => {
            b = b.header("content-type", "application/json");
            Body::from(j.to_string())
        }
        None => Body::empty(),
    };
    let resp = router.clone().oneshot(b.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, v)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_call_board_activates_an_agent_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("ac.db")).expect("store");

    // Scripted agent: its one turn commands a fan at priority 14 (allowed).
    let mut state = AppState {
        store: store.clone(),
        bus: None,
        query: None,
        his_tier: None,
        agent: None,
        ai_min_priority: 13,
        ai_escalation_floor: 1,
    };
    let script = [ProviderScriptEvent::ToolCall {
        id: "c1".into(),
        name: "rubix_write_point".into(),
        arguments: json!({"point": "ac/s1/ahu-3/fan", "value": true, "priority": 14}),
        tokens: Default::default(),
    }];
    let executor = Arc::new(ScriptedLlmExecutor::new(script));
    let runtime = Arc::new(
        build_runtime_with_executor(&state, "scripted", "test-model", "test-model", 4, executor)
            .expect("runtime"),
    );
    state.agent = Some(runtime);
    let router = app(state);

    // Provision the point the agent run will command.
    req(&router, "POST", "/api/v1/sites",
        Some(json!({"org": "ac", "slug": "s1", "display_name": "s1", "tags": {"site": true}}))).await;
    let (_, equip) = req(&router, "POST", "/api/v1/equips",
        Some(json!({"site_id": site_id(&router).await, "path": "ahu-3",
                    "display_name": "AHU 3", "tags": {"equip": true}}))).await;
    let equip_id = equip["id"].as_str().unwrap();
    let (_, point) = req(&router, "POST", "/api/v1/points",
        Some(json!({"equip_id": equip_id, "slug": "fan", "display_name": "fan",
                    "kind": "cmd", "tags": {"point": true}}))).await;
    let fan = point["point"]["id"].as_str().unwrap().to_string();

    // Store an agent_call board and run it by slug (HTTP path wires the agent).
    let board = json!({
        "slug": "assess", "display_name": "Assess",
        "trigger": {"kind": "manual"},
        "board": {
            "nodes": [{
                "id": "a1", "component": "agent_call",
                "config": {"prompt": "AHU-3 looks off; investigate and act."}
            }],
            "connections": []
        }
    });
    let (s, _) = req(&router, "POST", "/api/v1/boards", Some(board)).await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, _) = req(&router, "POST", "/api/v1/boards/assess/run", None).await;
    assert_eq!(s, StatusCode::OK);

    // The agent run (detached) should command the fan shortly.
    let mut commanded = false;
    for _ in 0..30 {
        let (_, p) = req(&router, "GET", &format!("/api/v1/points/{fan}"), None).await;
        if p["point"]["cur_value"] == json!(true) {
            commanded = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    assert!(commanded, "agent_call board did not activate a run that commanded the point");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn awaited_agent_call_drives_a_gated_write_within_the_board_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("await.db")).expect("store");

    // Scripted agent: one turn commands the fan at priority 14 (allowed), then
    // its final response is what the awaited node surfaces downstream.
    let mut state = AppState {
        store: store.clone(),
        bus: None,
        query: None,
        his_tier: None,
        agent: None,
        ai_min_priority: 13,
        ai_escalation_floor: 1,
    };
    let script = [
        ProviderScriptEvent::ToolCall {
            id: "c1".into(),
            name: "rubix_write_point".into(),
            arguments: json!({"point": "aw/s1/ahu-3/fan", "value": true, "priority": 14}),
            tokens: Default::default(),
        },
        ProviderScriptEvent::ChatResponse {
            content: "commanded the fan on".into(),
            tokens: Default::default(),
            finish_reason: StopReason::EndTurn,
        },
    ];
    let executor = Arc::new(ScriptedLlmExecutor::new(script));
    let runtime = Arc::new(
        build_runtime_with_executor(&state, "scripted", "test-model", "test-model", 4, executor)
            .expect("runtime"),
    );
    state.agent = Some(runtime);
    let router = app(state);

    req(&router, "POST", "/api/v1/sites",
        Some(json!({"org": "aw", "slug": "s1", "display_name": "s1", "tags": {"site": true}}))).await;
    let (_, equip) = req(&router, "POST", "/api/v1/equips",
        Some(json!({"site_id": site_id(&router).await, "path": "ahu-3",
                    "display_name": "AHU 3", "tags": {"equip": true}}))).await;
    let equip_id = equip["id"].as_str().unwrap();
    let (_, point) = req(&router, "POST", "/api/v1/points",
        Some(json!({"equip_id": equip_id, "slug": "fan", "display_name": "fan",
                    "kind": "cmd", "tags": {"point": true}}))).await;
    let fan = point["point"]["id"].as_str().unwrap().to_string();

    // An awaited agent_call: the board blocks on the run inside the single-shot
    // evaluation, so the write lands before `/run` returns.
    let board = json!({
        "slug": "assess-await", "display_name": "Assess",
        "trigger": {"kind": "manual"},
        "board": {
            "nodes": [{
                "id": "a1", "component": "agent_call",
                "config": {"prompt": "AHU-3 looks off; investigate and act.", "await": true}
            }],
            "connections": []
        }
    });
    let (s, _) = req(&router, "POST", "/api/v1/boards", Some(board)).await;
    assert_eq!(s, StatusCode::CREATED);
    let (s, run) = req(&router, "POST", "/api/v1/boards/assess-await/run", None).await;
    assert_eq!(s, StatusCode::OK);

    // The agent's decision is observable on the node's output (it awaited).
    let surfaced = run["outputs"]
        .as_array()
        .and_then(|outs| outs.iter().find(|o| o["node"] == "a1" && o["port"] == "output"));
    assert!(surfaced.is_some(), "awaited agent_call did not surface an output: {run}");

    // The gated write reached the store during the run — no post-run polling.
    let (_, p) = req(&router, "GET", &format!("/api/v1/points/{fan}"), None).await;
    assert_eq!(p["point"]["cur_value"], json!(true),
        "awaited agent_call run did not command the point within the board run");
}

async fn site_id(router: &Router) -> String {
    let (_, list) = req(router, "GET", "/api/v1/sites", None).await;
    list[0]["id"].as_str().unwrap().to_string()
}

/// A board access without an agent rejects `request_agent` — the guard that
/// stops the agent's own `run_board` tool from re-triggering the agent.
#[tokio::test]
async fn request_agent_fails_closed_without_an_agent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("guard.db")).expect("store");
    let access = StorePointAccess::new(store);
    let err = access
        .request_agent(AgentRequest {
            thread: "t".into(),
            prompt: "do something".into(),
        })
        .unwrap_err();
    assert!(err.to_string().contains("no agent runtime"), "{err}");
    // The node is still a registered component regardless.
    assert!(COMPONENTS.contains(&"agent_call"));
}
