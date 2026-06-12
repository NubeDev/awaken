//! Test harness: fresh on-disk store per test, requests via tower oneshot.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use rubix_server::store::Store;
use rubix_server::{app, AppState};
use serde_json::Value;
use tower::ServiceExt;

pub struct TestApp {
    router: Router,
    _dir: tempfile::TempDir,
}

impl TestApp {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("test.db")).expect("open store");
        let state = AppState {
            store,
            ai_min_priority: 13,
        };
        Self {
            router: app(state),
            _dir: dir,
        }
    }

    pub async fn request(
        &self,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        let body = match body {
            Some(json) => {
                builder = builder.header("content-type", "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .router
            .clone()
            .oneshot(builder.body(body).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).expect("json body")
        };
        (status, json)
    }

    pub async fn create_site(&self) -> String {
        let (status, body) = self
            .request(
                "POST",
                "/api/v1/sites",
                Some(serde_json::json!({
                    "org": "nube", "slug": "hq", "display_name": "HQ",
                    "tags": {"site": true}
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().expect("site id").to_string()
    }

    pub async fn create_equip(&self, site_id: &str) -> String {
        let (status, body) = self
            .request(
                "POST",
                "/api/v1/equips",
                Some(serde_json::json!({
                    "site_id": site_id, "path": "ahu-3", "display_name": "AHU 3",
                    "tags": {"ahu": true, "equip": true}
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().expect("equip id").to_string()
    }

    pub async fn create_point(&self, equip_id: &str, kind: &str, slug: &str) -> String {
        let (status, body) = self
            .request(
                "POST",
                "/api/v1/points",
                Some(serde_json::json!({
                    "equip_id": equip_id, "slug": slug, "display_name": slug,
                    "kind": kind, "unit": "°C",
                    "tags": {"temp": true, "point": true}
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["point"]["id"].as_str().expect("point id").to_string()
    }
}
