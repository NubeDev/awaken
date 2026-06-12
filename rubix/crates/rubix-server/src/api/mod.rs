//! HTTP API. One file per route verb; this module is router wiring only.

mod blocking;
mod boards;
mod command;
mod equips;
mod health;
mod his;
mod openapi;
mod points;
mod query;
mod sites;
mod sparks;
mod tag_query;

pub use openapi::ApiDoc;

use axum::routing::get;
use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/api-docs/openapi.json", get(openapi::openapi_json))
        .merge(sites::router())
        .merge(equips::router())
        .merge(points::router())
        .merge(command::router())
        .merge(his::router())
        .merge(sparks::router())
        .merge(query::router())
        .merge(boards::router())
        .with_state(state)
}
