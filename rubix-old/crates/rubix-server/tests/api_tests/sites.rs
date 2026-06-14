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
async fn patch_site_edits_metadata_keeps_identity() {
    let app = TestApp::new();
    let id = app.create_site().await;

    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/sites/{id}"),
            Some(json!({"display_name": "NUBE HQ", "tags": {"site": true, "hq": true}})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["display_name"], "NUBE HQ");
    assert_eq!(body["tags"]["hq"], true);
    // Identity fields untouched.
    assert_eq!(body["org"], "nube");
    assert_eq!(body["slug"], "hq");

    // Absent field = unchanged: patch only tags, display_name persists.
    let (status, body) = app
        .request(
            "PATCH",
            &format!("/api/v1/sites/{id}"),
            Some(json!({"tags": {"site": true}})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["display_name"], "NUBE HQ");
    assert_eq!(body["tags"].get("hq"), None);
}

#[tokio::test]
async fn patch_site_rejects_identity_change() {
    let app = TestApp::new();
    let id = app.create_site().await;
    let (status, _) = app
        .request(
            "PATCH",
            &format!("/api/v1/sites/{id}"),
            Some(json!({"org": "kfc"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn patch_missing_site_404() {
    let app = TestApp::new();
    let (status, _) = app
        .request(
            "PATCH",
            "/api/v1/sites/00000000-0000-0000-0000-000000000000",
            Some(json!({"display_name": "X"})),
        )
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
