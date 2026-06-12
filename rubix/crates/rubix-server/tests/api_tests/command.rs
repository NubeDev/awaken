use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

async fn writable_point(app: &TestApp) -> String {
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    app.create_point(&equip, "sp", "cooling-sp").await
}

#[tokio::test]
async fn operator_write_wins_over_agent_and_relinquish_restores() {
    let app = TestApp::new();
    let point = writable_point(&app).await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/points/{point}/write"),
            Some(json!({"value": 22.0, "priority": 13, "source": "agent"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["point"]["cur_value"], 22.0);

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/points/{point}/write"),
            Some(json!({"value": 18.0, "priority": 8})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["point"]["cur_value"], 18.0);

    let (status, body) = app
        .request("DELETE", &format!("/api/v1/points/{point}/write/8"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["point"]["cur_value"], 22.0);
}

#[tokio::test]
async fn agent_write_above_min_priority_is_forbidden() {
    let app = TestApp::new();
    let point = writable_point(&app).await;
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{point}/write"),
            Some(json!({"value": 10.0, "priority": 8, "source": "agent"})),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn sensor_cannot_be_commanded_and_writable_cannot_ingest_cur() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let sensor = app.create_point(&equip, "sensor", "room-temp").await;
    let sp = app.create_point(&equip, "sp", "room-sp").await;

    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{sensor}/write"),
            Some(json!({"value": 1.0})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{sp}/cur"),
            Some(json!({"value": 21.0})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sensor_cur_ingest_updates_current_value() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let sensor = app.create_point(&equip, "sensor", "room-temp").await;

    let (status, body) = app
        .request(
            "POST",
            &format!("/api/v1/points/{sensor}/cur"),
            Some(json!({"value": 21.4})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["cur_value"], 21.4);
}

#[tokio::test]
async fn write_out_of_range_priority_is_bad_request() {
    let app = TestApp::new();
    let point = writable_point(&app).await;
    let (status, _) = app
        .request(
            "POST",
            &format!("/api/v1/points/{point}/write"),
            Some(json!({"value": 1.0, "priority": 17})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
