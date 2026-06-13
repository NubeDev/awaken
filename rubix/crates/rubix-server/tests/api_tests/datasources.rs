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
