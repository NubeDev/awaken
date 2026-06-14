//! POST /api/v1/his/rollup — time-bucketed aggregates over the query engine.

use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn hourly_average_over_inserted_history() {
    let app = TestApp::with_query().await;
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let point = app.create_point(&equip, "sensor", "room-temp").await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/points/{point}/his"),
            Some(json!([
                {"ts": "2026-06-12T00:05:00Z", "value": 20.0},
                {"ts": "2026-06-12T00:35:00Z", "value": 22.0},
                {"ts": "2026-06-12T01:15:00Z", "value": 30.0}
            ])),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/his/rollup",
            Some(json!({
                "points": [point],
                "interval": "hour",
                "agg": "avg"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let series = body["series"].as_array().expect("series");
    assert_eq!(series.len(), 2);
    assert_eq!(series[0]["value"], 21.0);
    assert_eq!(series[0]["samples"], 2);
    assert_eq!(series[1]["value"], 30.0);
}

#[tokio::test]
async fn unavailable_when_engine_disabled() {
    let app = TestApp::new();
    let (status, _body) = app
        .request(
            "POST",
            "/api/v1/his/rollup",
            Some(json!({ "points": ["x"], "interval": "hour", "agg": "avg" })),
        )
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn injection_attempt_is_bad_request() {
    let app = TestApp::with_query().await;
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/his/rollup",
            Some(json!({
                "points": ["x' OR '1'='1"],
                "interval": "hour",
                "agg": "avg"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}
