//! Persistent run registry + HITL resume/cancel. A chat turn whose `write_point`
//! lands in the escalation band suspends; the run persists, lists, and an
//! operator resumes it (the held write reaches the point) or cancels it (the
//! point is untouched). A scripted executor drives the agent offline.

use std::sync::Arc;

use awaken_runtime::engine::{ProviderScriptEvent, ScriptedLlmExecutor};
use awaken_runtime_contract::contract::inference::StopReason;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rubix_server::agent::build_runtime_with_executor;
use rubix_server::store::Store;
use rubix_server::{app, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

/// An app whose agent issues one scripted `write_point` at `priority` (the
/// escalation band sits below the ceiling 13, above the floor 1, so priority 5
/// suspends). Returns the router and the tempdir keeping the db alive.
fn app_with_write(priority: u8) -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("runs.db")).expect("open store");
    let mut state = AppState {
        store,
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
            arguments: json!({
                "point": "nube/hq/ahu-3/fan", "value": true, "priority": priority
            }),
            tokens: Default::default(),
        },
        ProviderScriptEvent::ChatResponse {
            content: "done".into(),
            tokens: Default::default(),
            finish_reason: StopReason::EndTurn,
        },
    ];
    let executor = Arc::new(ScriptedLlmExecutor::new(script));
    let runtime =
        build_runtime_with_executor(&state, "scripted", "test-model", "test-model", 4, executor)
            .expect("build runtime");
    state.agent = Some(Arc::new(runtime));
    (app(state), dir)
}

async fn req(router: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(json) => {
            builder = builder.header("content-type", "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = router
        .clone()
        .oneshot(builder.body(body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("body").to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json")
    };
    (status, json)
}

/// Provision the site/equip/point the scripted write targets, over the router's
/// own store, so a resumed write resolves and lands.
async fn seed_fan(router: &Router) {
    let (s, site) = req(
        router,
        "POST",
        "/api/v1/sites",
        Some(json!({"org": "nube", "slug": "hq", "display_name": "hq", "tags": {"site": true}})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{site}");
    let (s, equip) = req(
        router,
        "POST",
        "/api/v1/equips",
        Some(json!({"site_id": site["id"], "path": "ahu-3", "display_name": "AHU 3",
                    "tags": {"equip": true}})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{equip}");
    let (s, point) = req(
        router,
        "POST",
        "/api/v1/points",
        Some(json!({"equip_id": equip["id"], "slug": "fan", "display_name": "fan",
                    "kind": "cmd", "tags": {"point": true}})),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{point}");
}

/// Suspend a run via the escalation band and return its id.
async fn suspend_run(router: &Router, thread: &str) -> String {
    let (status, body) = req(
        router,
        "POST",
        "/api/v1/agent/chat",
        Some(json!({"thread_id": thread, "message": "force the fan on"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], json!("awaiting_approval"), "{body}");
    body["run_id"].as_str().expect("run id").to_string()
}

#[tokio::test]
async fn suspended_run_persists_lists_and_is_fetchable() {
    let (router, _dir) = app_with_write(5);
    seed_fan(&router).await;
    let run_id = suspend_run(&router, "t-list").await;

    // Listed under the suspended filter, with the held write attached.
    let (status, list) = req(&router, "GET", "/api/v1/runs?status=suspended", None).await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let runs = list.as_array().expect("array");
    assert_eq!(runs.len(), 1, "{list}");
    assert_eq!(runs[0]["id"], json!(run_id));
    assert_eq!(runs[0]["origin"], json!("chat"));
    assert_eq!(runs[0]["pending_write"]["point"], json!("nube/hq/ahu-3/fan"));
    assert_eq!(runs[0]["pending_write"]["priority"], json!(5));

    // Fetchable by id.
    let (status, one) = req(&router, "GET", &format!("/api/v1/runs/{run_id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{one}");
    assert_eq!(one["status"], json!("suspended"));

    // A bogus id is 404.
    let (status, _) = req(&router, "GET", "/api/v1/runs/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn resume_applies_the_held_write_and_settles_the_run() {
    let (router, _dir) = app_with_write(5);
    seed_fan(&router).await;
    let run_id = suspend_run(&router, "t-resume").await;

    let (status, body) = req(&router, "POST", &format!("/api/v1/runs/{run_id}/resume"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["priority"], json!(5));
    assert_eq!(body["effective"], json!(true));

    // The point now holds the commanded value at the approved slot.
    let (_, points) = req(&router, "GET", "/api/v1/points", None).await;
    let fan = &points.as_array().expect("array")[0];
    assert_eq!(fan["cur_value"], json!(true), "{points}");

    // The run left suspended; a second resume is a conflict (one-shot approval).
    let (status, one) = req(&router, "GET", &format!("/api/v1/runs/{run_id}"), None).await;
    assert_eq!(one["status"], json!("resumed"), "{one}");
    let (status2, _) = req(&router, "POST", &format!("/api/v1/runs/{run_id}/resume"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status2, StatusCode::CONFLICT);
}

#[tokio::test]
async fn cancel_discards_the_held_write_and_leaves_the_point_untouched() {
    let (router, _dir) = app_with_write(5);
    seed_fan(&router).await;
    let run_id = suspend_run(&router, "t-cancel").await;

    let (status, _) = req(&router, "POST", &format!("/api/v1/runs/{run_id}/cancel"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The point was never commanded.
    let (_, points) = req(&router, "GET", "/api/v1/points", None).await;
    let fan = &points.as_array().expect("array")[0];
    assert!(fan["cur_value"].is_null(), "{points}");

    // The run records cancelled; resuming a cancelled run is a conflict.
    let (_, one) = req(&router, "GET", &format!("/api/v1/runs/{run_id}"), None).await;
    assert_eq!(one["status"], json!("cancelled"), "{one}");
    let (status, _) = req(&router, "POST", &format!("/api/v1/runs/{run_id}/resume"), None).await;
    assert_eq!(status, StatusCode::CONFLICT);
}
