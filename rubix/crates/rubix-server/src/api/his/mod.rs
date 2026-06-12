//! History routes — wiring only.

pub(crate) mod insert;
pub(crate) mod query;
pub(crate) mod rollup;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/points/{id}/his",
            get(query::query_his).post(insert::insert_his),
        )
        .route("/api/v1/his/rollup", post(rollup::rollup_his))
}
