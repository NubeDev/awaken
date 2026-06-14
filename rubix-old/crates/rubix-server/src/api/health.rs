use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct Health {
    pub status: &'static str,
    pub version: &'static str,
}

#[utoipa::path(get, path = "/healthz", tag = "meta", responses((status = 200, body = Health)))]
pub(super) async fn healthz() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
