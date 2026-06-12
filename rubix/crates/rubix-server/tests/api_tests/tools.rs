//! Agent tool set wired to server state: tools are built, discoverable, and
//! gated against a real store.

use awaken_runtime_contract::contract::tool::{Tool, ToolCallContext, ToolError};
use rubix_server::tools::{build_tools, build_tools_scoped};
use rubix_tools::TenantScope;
use serde_json::json;

use super::harness::TestApp;

fn find<'a>(tools: &'a [std::sync::Arc<dyn Tool>], id: &str) -> &'a dyn Tool {
    tools
        .iter()
        .find(|t| t.descriptor().id == id)
        .map(|t| t.as_ref())
        .unwrap_or_else(|| panic!("tool {id} not built"))
}

#[tokio::test]
async fn build_tools_exposes_point_tools_without_query_engine() {
    let (app, state) = TestApp::with_state();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let _point = app.create_point(&equip, "cmd", "fan").await;

    let tools = build_tools(&state);
    let ids: Vec<String> = tools.iter().map(|t| t.descriptor().id).collect();
    assert!(ids.contains(&"rubix_read_point".to_string()));
    assert!(ids.contains(&"rubix_write_point".to_string()));
    // No query engine configured → no query tool.
    assert!(!ids.contains(&"rubix_query".to_string()));

    // write_point commands a real point through the store.
    let write = find(&tools, "rubix_write_point");
    let out = write
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("write");
    assert_eq!(out.result.data["effective"], json!(true));

    // read_point sees the commanded value.
    let read = find(&tools, "rubix_read_point");
    let out = read
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("read");
    assert_eq!(out.result.data["value"], json!(true));
}

#[tokio::test]
async fn run_board_tool_evaluates_a_board_over_the_store() {
    let (app, state) = TestApp::with_state();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let temp = app.create_point(&equip, "sensor", "temp").await;
    let _fan = app.create_point(&equip, "cmd", "fan").await;

    // Seed the sensor so the board's read node has a value to forward.
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{temp}/cur"),
            Some(json!({ "value": 21.5 })),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::OK);

    let tools = build_tools(&state);
    let ids: Vec<String> = tools.iter().map(|t| t.descriptor().id).collect();
    assert!(ids.contains(&"rubix_run_board".to_string()));

    // read temp → command fan with the read value at priority 8.
    let run_board = find(&tools, "rubix_run_board");
    let out = run_board
        .execute(
            json!({ "board": {
                "nodes": [
                    {"id": "r1", "component": "read_point",
                     "config": {"point": "nube/hq/ahu-3/temp"}},
                    {"id": "w1", "component": "write_point",
                     "config": {"point": "nube/hq/ahu-3/fan", "priority": 8}}
                ],
                "connections": [
                    {"from_node": "r1", "from_port": "output",
                     "to_node": "w1", "to_port": "value"}
                ]
            }}),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("run_board");
    let outputs = out.result.data["outputs"].as_array().expect("outputs");
    assert!(outputs.iter().any(|o| o["node"] == "w1"));

    // The board committed 21.5 to the fan through the priority array.
    let read = find(&tools, "rubix_read_point");
    let read_out = read
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("read");
    assert_eq!(read_out.result.data["value"], json!(21.5));
}

#[tokio::test]
async fn write_tool_escalates_above_agent_ceiling() {
    let (app, state) = TestApp::with_state();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let _point = app.create_point(&equip, "cmd", "fan").await;

    let tools = build_tools(&state);
    let write = find(&tools, "rubix_write_point");
    // ai_min_priority is 13, escalation_floor is 1 in the harness; slot 5 is
    // above the agent ceiling but inside the escalation band → suspends for
    // approval rather than committing or denying.
    let out = write
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true, "priority": 5 }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("suspended output");
    assert!(out.result.is_pending());
    let ticket = out.result.suspension.expect("ticket");
    assert_eq!(ticket.suspension.action, "approve_write");

    // The point was never commanded while awaiting approval.
    let read = find(&tools, "rubix_read_point");
    let read_out = read
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("read");
    assert_eq!(read_out.result.data["value"], json!(null));
}

#[tokio::test]
async fn pin_widget_tool_persists_a_dashboard_tile() {
    let (app, state) = TestApp::with_state();
    let site = app.create_site().await;

    let tools = build_tools(&state);
    let ids: Vec<String> = tools.iter().map(|t| t.descriptor().id).collect();
    assert!(ids.contains(&"rubix_pin_widget".to_string()));

    let pin = find(&tools, "rubix_pin_widget");
    let out = pin
        .execute(
            json!({
                "site_id": site, "kind": "point_value",
                "title": "AHU-3 fan", "target": "nube/hq/ahu-3/fan"
            }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("pin");
    assert_eq!(out.result.data["kind"], json!("point_value"));

    // The pinned widget is listable over the HTTP surface.
    let (status, body) = app
        .request("GET", &format!("/api/v1/widgets?site_id={site}"), None)
        .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["target"], "nube/hq/ahu-3/fan");
}

#[tokio::test]
async fn pin_widget_tool_rejects_unknown_kind() {
    let (_app, state) = TestApp::with_state();
    let tools = build_tools(&state);
    let pin = find(&tools, "rubix_pin_widget");
    let err = pin
        .execute(
            json!({
                "site_id": "00000000-0000-0000-0000-000000000000",
                "kind": "bogus", "title": "t", "target": "x"
            }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::InvalidArguments(_)));
}

/// Seed two tenants on `app`, each with one equip and one point, so a scoped
/// `query` must filter the `sites`/`points` tables by `{org}/{site}`.
async fn seed_two_tenants(app: &TestApp) {
    let site_a = app.create_site_with("ten", "qa").await;
    let equip_a = app.create_equip(&site_a).await;
    app.create_point(&equip_a, "sensor", "temp").await;
    let site_b = app.create_site_with("ten", "qb").await;
    let equip_b = app.create_equip(&site_b).await;
    app.create_point(&equip_b, "sensor", "temp").await;
}

/// An unscoped build gets the full `query` tool that sees every tenant's rows.
#[tokio::test]
async fn unscoped_query_tool_sees_all_tenants() {
    let (app, state) = TestApp::with_state_query().await;
    seed_two_tenants(&app).await;

    let tools = build_tools_scoped(&state, None);
    let query = find(&tools, "rubix_query");
    let out = query
        .execute(
            json!({ "sql": "SELECT org, slug FROM sites ORDER BY slug" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("query");
    assert_eq!(out.result.data["row_count"], json!(2));
}

/// A scoped build keeps the `query` tool, but it runs through a tenant-filtered
/// session: SQL over `sites` returns only the run's own `{org}/{site}`, even
/// when the SQL explicitly names the sibling tenant.
#[tokio::test]
async fn scoped_query_tool_sees_only_its_tenant() {
    let (app, state) = TestApp::with_state_query().await;
    seed_two_tenants(&app).await;

    let scope = TenantScope::new("ten", "qa");
    let tools = build_tools_scoped(&state, Some(scope));
    // The tool is present (no longer withheld for scoped runs).
    let ids: Vec<String> = tools.iter().map(|t| t.descriptor().id).collect();
    assert!(ids.contains(&"rubix_query".to_string()), "scoped run lost its query tool");

    let query = find(&tools, "rubix_query");

    // A bare select over sites sees only the scoped tenant.
    let out = query
        .execute(
            json!({ "sql": "SELECT org, slug FROM sites" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("query");
    assert_eq!(out.result.data["row_count"], json!(1));
    assert_eq!(out.result.data["rows"][0]["slug"], json!("qa"));

    // Explicitly naming the sibling tenant cannot escape the filter.
    let out = query
        .execute(
            json!({ "sql": "SELECT slug FROM sites WHERE slug = 'qb'" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("query");
    assert_eq!(out.result.data["row_count"], json!(0));

    // points_cur keyexprs stay inside the tenant.
    let out = query
        .execute(
            json!({ "sql": "SELECT keyexpr FROM points_cur" }),
            &ToolCallContext::test_default(),
        )
        .await
        .expect("query");
    assert_eq!(out.result.data["row_count"], json!(1));
    assert_eq!(out.result.data["rows"][0]["keyexpr"], json!("ten/qa/ahu-3/temp"));
}
