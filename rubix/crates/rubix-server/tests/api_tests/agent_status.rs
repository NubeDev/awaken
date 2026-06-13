//! GET /api/v1/agent/status — the read-only agent config view backing the
//! operator's "Agent status" panel. Disabled by default; reports provider/model
//! and the priority gate when the agent is embedded.

use std::sync::Arc;

use awaken_runtime::engine::{ProviderScriptEvent, ScriptedLlmExecutor};
use awaken_runtime_contract::contract::inference::StopReason;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rubix_server::agent::build_runtime_with_executor;
use rubix_server::profile::{Profile, ProfileKind};
use rubix_server::store::Store;
use rubix_server::{app, AppState};
use serde_json::Value;
use tower::ServiceExt;

use super::harness::TestApp;

async fn get(router: &Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request");
    let response = router.clone().oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).expect("json");
    (status, json)
}

#[tokio::test]
async fn status_reports_disabled_without_agent() {
    // The default harness has no agent (RUBIX_AI off equivalent).
    let app = TestApp::new();
    let (status, body) = app.request("GET", "/api/v1/agent/status", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], false);
    // Model fields are omitted when disabled.
    assert!(body.get("provider").is_none());
    assert!(body.get("model").is_none());
    assert!(body.get("max_rounds").is_none());
    // The gate is always reported (it is configured even when the agent is off).
    assert_eq!(body["min_priority"], 13);
    assert_eq!(body["escalation_floor"], 1);
    assert_eq!(body["dispatch_ready"], false);
}

#[tokio::test]
async fn status_reports_config_when_agent_embedded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("test.db")).expect("open store");
    let mut state = AppState {
        profile: Profile::defaults(ProfileKind::Edge),
        store,
        bus: None,
        query: None,
        his_tier: None,
        agent: None,
        agent_blueprint: None,
        ai_min_priority: 12,
        ai_escalation_floor: 5,
        authenticator: None,
        scheduler: None,
        datasources: None,
    };
    let executor = Arc::new(ScriptedLlmExecutor::new([ProviderScriptEvent::ChatResponse {
        content: "ok".to_string(),
        tokens: Default::default(),
        finish_reason: StopReason::EndTurn,
    }]));
    let runtime =
        build_runtime_with_executor(&state, "openai", "gpt-4o-mini", "gpt-4o-mini", 6, executor)
            .expect("build runtime");
    state.agent = Some(Arc::new(runtime));
    state.agent_blueprint = Some(rubix_server::agent::RuntimeBlueprint::genai(
        "openai",
        "gpt-4o-mini",
        "gpt-4o-mini",
        6,
    ));
    let router = app(state);

    let (status, body) = get(&router, "/api/v1/agent/status").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["enabled"], true);
    assert_eq!(body["provider"], "openai");
    assert_eq!(body["model"], "gpt-4o-mini");
    assert_eq!(body["max_rounds"], 6);
    assert_eq!(body["min_priority"], 12);
    assert_eq!(body["escalation_floor"], 5);
    // No bus wired in this state → dispatch not ready even with the agent up.
    assert_eq!(body["dispatch_ready"], false);
}
