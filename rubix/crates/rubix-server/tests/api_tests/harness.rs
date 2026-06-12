//! Test harness: fresh on-disk store per test, requests via tower oneshot.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use jsonwebtoken::jwk::JwkSet;
use rubix_query::{HisTier, QueryEngine};
use rubix_server::auth::{Authenticator, JwksVerifier};
use rubix_server::bus::ZenohBus;
use rubix_server::profile::{Profile, ProfileKind};
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
        Self::build(None)
    }

    /// Build and also hand back a clone of the store, for tests that exercise
    /// store-backed integrations (e.g. the flow `PointAccess`) directly.
    pub fn with_store() -> (Self, Store) {
        let (app, state) = Self::with_state();
        (app, state.store.clone())
    }

    /// Build and hand back a clone of the `AppState`, for tests that drive
    /// server-state integrations (e.g. the agent tool registry) directly.
    pub fn with_state() -> (Self, AppState) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("test.db")).expect("open store");
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store,
            bus: None,
            query: None,
            his_tier: None,
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: None,
        };
        let app = Self {
            router: app(state.clone()),
            _dir: dir,
        };
        (app, state)
    }

    /// Build with a DataFusion query engine over the same store and return the
    /// `AppState`, for tests that build the agent tool set (`build_tools_scoped`)
    /// against a live query surface.
    pub async fn with_state_query() -> (Self, AppState) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("test.db");
        let store = Store::open(&db).expect("open store");
        let query = QueryEngine::open(&db).await.expect("open query engine");
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store,
            bus: None,
            query: Some(query),
            his_tier: None,
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: None,
        };
        let app = Self {
            router: app(state.clone()),
            _dir: dir,
        };
        (app, state)
    }

    /// Build over an already-open store, for tests that drive a store-backed
    /// component (e.g. the scheduler) alongside the HTTP API on one DB.
    pub fn with_store_at(store: Store) -> Self {
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store,
            bus: None,
            query: None,
            his_tier: None,
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: None,
        };
        Self {
            router: app(state),
            _dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    /// Build with a live zenoh bus whose queryables serve the same store.
    pub async fn with_bus() -> (Self, ZenohBus) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("test.db")).expect("open store");
        let bus = ZenohBus::open(store.clone()).await.expect("open bus");
        bus.serve().await.expect("serve bus");
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store,
            bus: Some(bus.clone()),
            query: None,
            his_tier: None,
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: None,
        };
        let app = Self {
            router: app(state),
            _dir: dir,
        };
        (app, bus)
    }

    /// Build with a DataFusion query engine over the same store, so `/query`
    /// resolves the canonical tables.
    pub async fn with_query() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("test.db");
        let store = Store::open(&db).expect("open store");
        let query = QueryEngine::open(&db).await.expect("open query engine");
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store,
            bus: None,
            query: Some(query),
            his_tier: None,
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: None,
        };
        Self {
            router: app(state),
            _dir: dir,
        }
    }

    /// Build with both a query engine and a Parquet `his` cold tier over the
    /// same store, so `/his/flush` ages rows into Parquet and `/query` reads
    /// them back across the tier boundary.
    pub async fn with_query_tier() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("test.db");
        let store = Store::open(&db).expect("open store");
        let tier = HisTier::open_local(&dir.path().join("his-parquet")).expect("open tier");
        let query = QueryEngine::open(&db)
            .await
            .expect("open query engine")
            .with_his_tier(tier.clone());
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store,
            bus: None,
            query: Some(query),
            his_tier: Some(tier),
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: None,
        };
        Self {
            router: app(state),
            _dir: dir,
        }
    }

    /// Build with auth enforced (cloud posture). The JWKS is empty, so OIDC JWT
    /// verification always fails closed — tests drive the PAT path, which needs
    /// no key material. Hands back the store so a test can seed a PAT directly.
    pub fn with_auth() -> (Self, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("test.db")).expect("open store");
        let authenticator = Authenticator::new(
            JwksVerifier::from_keys(JwkSet { keys: vec![] }, "https://issuer.test"),
            store.clone(),
        );
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store: store.clone(),
            bus: None,
            query: None,
            his_tier: None,
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: Some(authenticator),
        };
        let app = Self {
            router: app(state),
            _dir: dir,
        };
        (app, store)
    }

    /// Like [`request`](Self::request) but presents a bearer token.
    pub async fn request_as(
        &self,
        method: &str,
        uri: &str,
        bearer: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {bearer}"));
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

    fn build(bus: Option<ZenohBus>) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("test.db")).expect("open store");
        let state = AppState {
            profile: Profile::defaults(ProfileKind::Edge),
            store,
            bus,
            query: None,
            his_tier: None,
            agent: None,
            agent_blueprint: None,
            ai_min_priority: 13,
            ai_escalation_floor: 1,
            authenticator: None,
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
        self.create_site_with("nube", "hq").await
    }

    /// Create a site with an explicit org/slug so concurrent bus tests get
    /// non-colliding keyexprs on the shared zenoh mesh.
    pub async fn create_site_with(&self, org: &str, slug: &str) -> String {
        let (status, body) = self
            .request(
                "POST",
                "/api/v1/sites",
                Some(serde_json::json!({
                    "org": org, "slug": slug, "display_name": slug,
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
