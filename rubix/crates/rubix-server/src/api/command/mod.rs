//! Point command routes (priority-array write path) — wiring only.

pub(crate) mod apply;
pub(crate) mod cur;
pub(crate) mod relinquish;
pub(crate) mod source;
pub(crate) mod write;

use axum::routing::{delete, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/points/{id}/write", post(write::write_point))
        .route(
            "/api/v1/points/{id}/write/{priority}",
            delete(relinquish::relinquish_point),
        )
        .route("/api/v1/points/{id}/cur", post(cur::ingest_cur))
}
