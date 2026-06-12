use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn create_and_list_widget() {
    let app = TestApp::new();
    let site = app.create_site().await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_value",
                "title": "AHU-3 fan", "target": "nube/hq/ahu-3/fan"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["kind"], "point_value");

    let (status, body) = app
        .request("GET", &format!("/api/v1/widgets?site_id={site}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["title"], "AHU-3 fan");
    assert_eq!(body[0]["target"], "nube/hq/ahu-3/fan");
}

#[tokio::test]
async fn widget_for_missing_site_is_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": "00000000-0000-0000-0000-000000000000",
                "kind": "board_output", "title": "t", "target": "b1"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn empty_title_is_400() {
    let app = TestApp::new();
    let site = app.create_site().await;
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/widgets",
            Some(json!({
                "site_id": site, "kind": "point_value", "title": "  ", "target": "x"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
