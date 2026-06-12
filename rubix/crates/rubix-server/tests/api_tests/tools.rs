//! Agent tool set wired to server state: tools are built, discoverable, and
//! gated against a real store.

use awaken_runtime_contract::contract::tool::{Tool, ToolCallContext, ToolError};
use rubix_server::tools::build_tools;
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
async fn write_tool_enforces_agent_priority_gate() {
    let (app, state) = TestApp::with_state();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let _point = app.create_point(&equip, "cmd", "fan").await;

    let tools = build_tools(&state);
    let write = find(&tools, "rubix_write_point");
    // ai_min_priority is 13 in the harness; slot 5 is above the agent ceiling.
    let err = write
        .execute(
            json!({ "point": "nube/hq/ahu-3/fan", "value": true, "priority": 5 }),
            &ToolCallContext::test_default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::Denied(_)));
}
