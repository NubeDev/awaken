//! POST /api/v1/query — DataFusion SQL surface over the store.

use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn selects_rows_created_through_the_api() {
    let app = TestApp::with_query().await;
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    app.create_point(&equip, "sensor", "temp").await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/query",
            Some(json!({ "sql": "SELECT slug FROM points ORDER BY slug" })),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let rows = body["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["slug"], "temp");
}

#[tokio::test]
async fn invalid_sql_is_bad_request() {
    let app = TestApp::with_query().await;

    let (status, _body) = app
        .request(
            "POST",
            "/api/v1/query",
            Some(json!({ "sql": "SELECT * FROM no_such_table" })),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unavailable_when_engine_disabled() {
    let app = TestApp::new();

    let (status, _body) = app
        .request("POST", "/api/v1/query", Some(json!({ "sql": "SELECT 1" })))
        .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}
