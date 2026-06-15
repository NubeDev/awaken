//! Integration: `GET /health` reports ok over the assembled transport.

#[path = "../fixture/mod.rs"]
mod fixture;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use fixture::app::boot;

#[tokio::test]
async fn health_route_reports_ok() {
    let app = boot("server_health", &[]).await.app;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("route responds");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.expect("body").to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");
    assert_eq!(json["status"], "ok");
}
