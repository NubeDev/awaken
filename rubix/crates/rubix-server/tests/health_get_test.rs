//! Integration: `GET /health` returns 200 with an ok status over the router.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rubix_core::RuntimeConfig;
use rubix_server::{AppState, router};
use rubix_store::StoreHandle;
use tower::ServiceExt;

#[tokio::test]
async fn health_route_reports_ok() {
    let store = StoreHandle::open(&RuntimeConfig::in_memory("rubix", "server_health"))
        .await
        .expect("open store");
    let app = router(AppState::new(store));

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
