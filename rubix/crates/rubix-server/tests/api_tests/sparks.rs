use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn create_list_ack_spark() {
    let app = TestApp::new();
    let site = app.create_site().await;

    let (status, body) = app
        .request(
            "POST",
            "/api/v1/sparks",
            Some(json!({
                "site_id": site, "rule": "simultaneous-heat-cool",
                "severity": "fault", "message": "AHU-3 heating and cooling at once"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["id"].as_str().unwrap().to_string();

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/sparks?site_id={site}&acknowledged=false"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["severity"], "fault");

    let (status, _) = app
        .request("POST", &format!("/api/v1/sparks/{id}/ack"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = app
        .request(
            "GET",
            &format!("/api/v1/sparks?site_id={site}&acknowledged=false"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn spark_for_missing_site_is_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/sparks",
            Some(json!({
                "site_id": "00000000-0000-0000-0000-000000000000",
                "rule": "r1", "severity": "info", "message": "m"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
