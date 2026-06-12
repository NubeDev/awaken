use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn batch_insert_then_range_query() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let point = app.create_point(&equip, "sensor", "room-temp").await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/points/{point}/his"),
            Some(json!([
                {"ts": "2026-06-12T00:00:00Z", "value": 20.0},
                {"ts": "2026-06-12T01:00:00Z", "value": 21.0},
                {"ts": "2026-06-12T02:00:00Z", "value": 22.0}
            ])),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["inserted"], 3);

    let (status, body) = app
        .request(
            "GET",
            &format!(
                "/api/v1/points/{point}/his?start=2026-06-12T00:30:00Z&end=2026-06-12T02:00:00Z"
            ),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let samples = body.as_array().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0]["value"], 21.0);
}

#[tokio::test]
async fn command_writes_land_in_history() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let point = app.create_point(&equip, "sp", "cooling-sp").await;

    app.request(
        "POST",
        &format!("/api/v1/points/{point}/write"),
        Some(json!({"value": 23.0, "priority": 16})),
    )
    .await;

    let (status, body) = app
        .request("GET", &format!("/api/v1/points/{point}/his"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let samples = body.as_array().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0]["value"], 23.0);
}

#[tokio::test]
async fn flush_ages_rows_into_parquet_and_query_still_reads_them() {
    let app = TestApp::with_query_tier().await;
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let point = app.create_point(&equip, "sensor", "room-temp").await;

    app.request(
        "POST",
        &format!("/api/v1/points/{point}/his"),
        Some(json!([
            {"ts": "2026-01-01T00:00:00Z", "value": 10.0},
            {"ts": "2026-01-02T00:00:00Z", "value": 20.0}
        ])),
    )
    .await;

    // Flush everything older than now into the Parquet cold tier.
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/his/flush",
            Some(json!({"older_than": "2026-06-12T00:00:00Z"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["rows"], 2);
    assert_eq!(body["partitions"], 2, "one partition per UTC day");

    // The SQLite hot tier no longer holds the rows.
    let (status, body) = app
        .request("GET", &format!("/api/v1/points/{point}/his"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_array().unwrap().is_empty(), "hot tier drained");

    // The union query surface still returns them from the cold tier.
    let (status, body) = app
        .request(
            "POST",
            "/api/v1/query",
            Some(json!({
                "sql": format!("SELECT value FROM his WHERE point_id = '{point}' ORDER BY ts")
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows = body["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "cold-tier rows read via the union provider");
    assert_eq!(rows[0]["value"], "10.0");
    assert_eq!(rows[1]["value"], "20.0");
}

#[tokio::test]
async fn flush_without_a_tier_is_503() {
    let app = TestApp::new();
    let (status, _) = app
        .request("POST", "/api/v1/his/flush", Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn his_on_missing_point_is_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "GET",
            "/api/v1/points/00000000-0000-0000-0000-000000000000/his",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
