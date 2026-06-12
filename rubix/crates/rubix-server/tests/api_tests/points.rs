use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn create_point_returns_keyexpr_identity() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let point = app.create_point(&equip, "sensor", "discharge-temp").await;

    let (status, body) = app
        .request("GET", &format!("/api/v1/points/{point}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["keyexpr"], "nube/hq/ahu-3/discharge-temp");
    assert_eq!(body["point"]["kind"], "sensor");
}

#[tokio::test]
async fn list_points_filters_by_tags_and_site() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    app.create_point(&equip, "sensor", "discharge-temp").await;
    app.create_point(&equip, "sp", "cooling-sp").await;

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/points?site_id={site}&tags=temp"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/points?site_id={site}&tags=nonexistent"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn sensor_rejects_relinquish_default() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/points",
            Some(json!({
                "equip_id": equip, "slug": "rt", "display_name": "Room Temp",
                "kind": "sensor", "relinquish_default": 20.0
            })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_point_under_missing_equip_is_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/points",
            Some(json!({
                "equip_id": "00000000-0000-0000-0000-000000000000",
                "slug": "x", "display_name": "X", "kind": "sensor"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
