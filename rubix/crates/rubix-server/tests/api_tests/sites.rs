use axum::http::StatusCode;
use serde_json::json;

use super::harness::TestApp;

#[tokio::test]
async fn create_get_list_delete_site() {
    let app = TestApp::new();
    let id = app.create_site().await;

    let (status, body) = app
        .request("GET", &format!("/api/v1/sites/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["slug"], "hq");
    assert_eq!(body["tags"]["site"], true);

    let (status, body) = app.request("GET", "/api/v1/sites?org=nube", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (status, _) = app
        .request("DELETE", &format!("/api/v1/sites/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = app
        .request("GET", &format!("/api/v1/sites/{id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn duplicate_site_slug_conflicts() {
    let app = TestApp::new();
    app.create_site().await;
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/sites",
            Some(json!({"org": "nube", "slug": "hq", "display_name": "Dup"})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn rejects_bad_slug() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "POST",
            "/api/v1/sites",
            Some(json!({"org": "nube", "slug": "HQ One", "display_name": "Bad"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn openapi_and_health_served() {
    let app = TestApp::new();
    let (status, body) = app.request("GET", "/api-docs/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["paths"]["/api/v1/points/{id}/write"].is_object());

    let (status, body) = app.request("GET", "/healthz", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}
