//! Embedded agent integration: the chat endpoint runs an awaken agent loop
//! built over the BMS tool set. A scripted LLM executor drives the loop offline
//! (no API key), proving the runtime is wired end-to-end through HTTP.

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

/// Build an app whose agent runtime is driven by a scripted executor that
/// replies with a fixed assistant turn. The BMS tools are still registered, so
/// this exercises the real `build_tools` → runtime path.
fn app_with_scripted_agent(reply: &str) -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("test.db")).expect("open store");
    let mut state = AppState {
        store,
        bus: None,
        query: None,
        agent: None,
        ai_min_priority: 13,
    };
    let executor = Arc::new(ScriptedLlmExecutor::new([
        ProviderScriptEvent::ChatResponse {
            content: reply.to_string(),
            tokens: Default::default(),
            finish_reason: StopReason::EndTurn,
        },
    ]));
    let runtime =
        build_runtime_with_executor(&state, "scripted", "test-model", "test-model", 4, executor)
            .expect("build runtime");
    state.agent = Some(Arc::new(runtime));
    (app(state), dir)
}

async fn post(router: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    let response = router.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("json")
    };
    (status, json)
}

#[tokio::test]
async fn chat_runs_the_agent_loop_and_returns_a_response() {
    let (router, _dir) = app_with_scripted_agent("AHU-3 looks nominal.");
    let (status, body) = post(
        &router,
        "/api/v1/agent/chat",
        json!({"thread_id": "t1", "message": "How is AHU-3?"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["response"], json!("AHU-3 looks nominal."));
}

#[tokio::test]
async fn chat_is_unavailable_when_agent_disabled() {
    // Default harness state has no agent → 503, mirroring the query route.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("test.db")).expect("open store");
    let state = AppState {
        store,
        bus: None,
        query: None,
        agent: None,
        ai_min_priority: 13,
    };
    let router = app(state);
    let (status, _) = post(
        &router,
        "/api/v1/agent/chat",
        json!({"thread_id": "t1", "message": "hi"}),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}
