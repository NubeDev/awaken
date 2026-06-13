//! External datasource routes — `/api/v1/datasources/{id}/…`.
//!
//! The exercised path here is the *absent* surface: with no `datasources.json`
//! loaded (`AppState.datasources == None`, the harness default), every
//! datasource route returns 503 rather than 404 or 500, so a deployment that
//! declares no datasource simply has no datasource surface. The live read/cap/
//! describe behavior is covered against a real Postgres by the `rubix-datasource`
//! crate's `#[ignore]`d `tests/live_postgres.rs`; wiring a live datasource into
//! the HTTP harness would need the same throwaway Postgres and is out of scope
//! for the in-repo unit gates.

use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn query_unavailable_when_no_datasources_configured() {
    let app = TestApp::new();

    let (status, _body) = app
        .request(
            "POST",
            "/api/v1/datasources/historian/query",
            Some(json!({ "sql": "SELECT 1" })),
        )
        .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn named_query_unavailable_when_no_datasources_configured() {
    let app = TestApp::new();

    let (status, _body) = app
        .request(
            "POST",
            "/api/v1/datasources/historian/named/site_daily",
            Some(json!({ "params": [] })),
        )
        .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn describe_unavailable_when_no_datasources_configured() {
    let app = TestApp::new();

    let (status, _body) = app
        .request("GET", "/api/v1/datasources/historian/describe", None)
        .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

/// A board `datasource` node fails closed when no datasource registry is wired:
/// the run succeeds (200) but the node emits on its `error` port rather than
/// `output`, so a spark folding the (absent) grid into a finding never fires.
/// This is the board-side mirror of the routes' 503.
#[tokio::test]
async fn datasource_board_node_fails_closed_without_a_registry() {
    let app = TestApp::new();

    let board = json!({
        "board": {
            "nodes": [
                {"id": "ds", "component": "datasource",
                 "config": {"datasource": "historian", "sql": "SELECT 1"}}
            ],
            "connections": []
        }
    });
    let (status, body) = app.request("POST", "/api/v1/boards/run", Some(board)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let outputs = body["outputs"].as_array().expect("outputs array");
    let ds = outputs
        .iter()
        .find(|o| o["node"] == "ds")
        .expect("datasource node output present");
    assert_eq!(ds["port"], "error", "fails closed on the error port: {ds}");
}
