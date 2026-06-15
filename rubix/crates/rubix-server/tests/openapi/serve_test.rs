//! Integration: `/api-docs/openapi.json` serves a document listing the routes.

#[path = "../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use fixture::app::boot;

#[tokio::test]
async fn openapi_lists_the_registered_routes() {
    let app = boot("server_openapi", &[]).await.app;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("route responds");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.expect("body").to_bytes();
    let doc: Value = serde_json::from_slice(&body).expect("json document");

    assert_eq!(doc["openapi"].as_str().expect("openapi version").chars().next(), Some('3'));
    let paths = doc["paths"].as_object().expect("paths object");
    for route in [
        "/health",
        "/records",
        "/records/{id}",
        "/query",
        "/datasources",
        "/ws/records",
    ] {
        assert!(paths.contains_key(route), "openapi missing {route}: {:?}", paths.keys());
    }
}
