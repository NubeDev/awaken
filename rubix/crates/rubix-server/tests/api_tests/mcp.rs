//! Outbound MCP adapter: external agents reach the BMS tools over JSON-RPC with
//! the same priority-array gating and HITL escalation as the embedded agent.

use awaken_runtime_contract::contract::tool::ToolCallContext;
use axum::http::StatusCode;
use rubix_server::tools::build_tools_scoped;
use rubix_tools::TenantScope;
use serde_json::json;

use super::harness::TestApp;

/// One JSON-RPC call against the MCP endpoint, returning the parsed response.
async fn rpc(app: &TestApp, id: i64, method: &str, params: serde_json::Value) -> serde_json::Value {
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/mcp",
            Some(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

#[tokio::test]
async fn tools_list_advertises_the_bms_surface() {
    let (app, _state) = TestApp::with_state();
    let body = rpc(&app, 1, "tools/list", json!({})).await;
    let names: Vec<&str> = body["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    assert!(names.contains(&"read_point"));
    assert!(names.contains(&"write_point"));
    assert!(names.contains(&"run_board"));
    // Each tool carries an input schema for the calling agent.
    let write = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "write_point")
        .unwrap();
    assert_eq!(write["inputSchema"]["type"], "object");
}

#[tokio::test]
async fn tools_call_writes_through_the_priority_array() {
    let (app, _state) = TestApp::with_state();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    app.create_point(&equip, "cmd", "fan").await;

    // A default-priority (slot 16) write is agent-eligible and commits.
    let body = rpc(
        &app,
        2,
        "tools/call",
        json!({"name": "write_point", "arguments": {
            "point": "nube/hq/ahu-3/fan", "value": true
        }}),
    )
    .await;
    assert_eq!(body["result"]["isError"], false, "{body}");
    assert_eq!(
        body["result"]["structuredContent"]["effective"],
        json!(true)
    );

    // read_point sees the commanded value, confirming the call hit the store.
    let body = rpc(
        &app,
        3,
        "tools/call",
        json!({"name": "read_point", "arguments": {"point": "nube/hq/ahu-3/fan"}}),
    )
    .await;
    assert_eq!(body["result"]["structuredContent"]["value"], json!(true));
}

#[tokio::test]
async fn below_floor_write_is_refused() {
    // Floor 5: slots 1..=4 are operator-reserved — refused even with approval.
    let app = TestApp::with_escalation_floor(5);
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    app.create_point(&equip, "cmd", "fan").await;

    let body = rpc(
        &app,
        4,
        "tools/call",
        json!({"name": "write_point", "arguments": {
            "point": "nube/hq/ahu-3/fan", "value": true, "priority": 3
        }}),
    )
    .await;
    // A hard refusal is a tool error result (no suspension, store untouched).
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert!(body["result"]["structuredContent"]["run_id"].is_null());

    // The point was not commanded.
    let body = rpc(
        &app,
        41,
        "tools/call",
        json!({"name": "read_point", "arguments": {"point": "nube/hq/ahu-3/fan"}}),
    )
    .await;
    assert_eq!(body["result"]["structuredContent"]["value"], json!(null));
}

#[tokio::test]
async fn escalation_band_write_suspends_into_the_run_registry() {
    let (app, _state) = TestApp::with_state();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    app.create_point(&equip, "cmd", "fan").await;

    // ai_min_priority is 13, floor is 1: slot 5 is above the agent ceiling but
    // inside the escalation band → the write suspends for operator approval.
    let body = rpc(
        &app,
        5,
        "tools/call",
        json!({"name": "write_point", "arguments": {
            "point": "nube/hq/ahu-3/fan", "value": true, "priority": 5
        }}),
    )
    .await;
    assert_eq!(body["result"]["isError"], false, "{body}");
    assert_eq!(
        body["result"]["structuredContent"]["status"],
        json!("awaiting_approval")
    );
    let run_id = body["result"]["structuredContent"]["run_id"]
        .as_str()
        .expect("run id")
        .to_string();

    // The point was not commanded while awaiting approval.
    let body = rpc(
        &app,
        6,
        "tools/call",
        json!({"name": "read_point", "arguments": {"point": "nube/hq/ahu-3/fan"}}),
    )
    .await;
    assert_eq!(body["result"]["structuredContent"]["value"], json!(null));

    // The suspended run appears in the operator surface, origin `mcp`.
    let (status, runs) = app
        .request("GET", "/api/v1/runs?status=suspended", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let run = runs
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == run_id)
        .expect("suspended run listed");
    assert_eq!(run["origin"], json!("mcp"));
    assert_eq!(run["pending_write"]["priority"], json!(5));

    // The operator resumes it; the held write commits through the priority array.
    let (status, resumed) = app
        .request("POST", &format!("/api/v1/runs/{run_id}/resume"), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{resumed}");
    assert_eq!(resumed["priority"], json!(5));

    let body = rpc(
        &app,
        7,
        "tools/call",
        json!({"name": "read_point", "arguments": {"point": "nube/hq/ahu-3/fan"}}),
    )
    .await;
    assert_eq!(body["result"]["structuredContent"]["value"], json!(true));
}

#[tokio::test]
async fn unknown_method_is_a_json_rpc_error() {
    let (app, _state) = TestApp::with_state();
    let body = rpc(&app, 8, "tools/unknown", json!({})).await;
    assert_eq!(body["error"]["code"], json!(-32601));
}

#[tokio::test]
async fn unknown_tool_is_an_error_result_not_a_transport_error() {
    let (app, _state) = TestApp::with_state();
    let body = rpc(
        &app,
        9,
        "tools/call",
        json!({"name": "no_such_tool", "arguments": {}}),
    )
    .await;
    // The JSON-RPC call succeeds; the tool error rides inside the result.
    assert!(body["error"].is_null());
    assert_eq!(body["result"]["isError"], true);
}

/// The adapter binds tool scope from the caller's principal: an MCP session
/// pinned to site A reaches the same `build_tools_scoped` registry the HTTP
/// handler builds, so a write to site B is refused at the tool boundary. (Auth
/// wiring that attaches the principal is exercised by the `auth` suite; this
/// proves the adapter's scope binding refuses the cross-tenant command.)
#[tokio::test]
async fn scoped_session_cannot_write_another_tenant() {
    let (app, state) = TestApp::with_state();
    let site_a = app.create_site_with("ten", "ma").await;
    let equip_a = app.create_equip(&site_a).await;
    app.create_point(&equip_a, "cmd", "fan").await;
    let site_b = app.create_site_with("ten", "mb").await;
    let equip_b = app.create_equip(&site_b).await;
    let fan_b = app.create_point(&equip_b, "cmd", "fan").await;

    // The handler builds tools scoped to the principal's site; replicate that
    // for a site-A session and attempt the cross-tenant write the adapter would
    // dispatch.
    let tools = build_tools_scoped(&state, Some(TenantScope::new("ten", "ma")));
    let write = tools
        .iter()
        .find(|t| t.descriptor().name == "write_point")
        .expect("write tool");
    let err = write
        .execute(
            json!({ "point": "ten/mb/ahu-3/fan", "value": true }),
            &ToolCallContext::test_default(),
        )
        .await;
    assert!(err.is_err(), "cross-tenant write was not refused");

    // Site B's point stayed untouched.
    let (_, body) = app
        .request("GET", &format!("/api/v1/points/{fan_b}"), None)
        .await;
    assert!(body["point"]["cur_value"].is_null());
}

#[tokio::test]
async fn initialize_handshake_advertises_tools_capability() {
    let (app, _state) = TestApp::with_state();
    let body = rpc(&app, 10, "initialize", json!({})).await;
    assert!(body["result"]["capabilities"]["tools"].is_object());
    assert_eq!(body["result"]["serverInfo"]["name"], "rubix-bms");
}
