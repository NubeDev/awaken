//! Spark finding routes — wiring only.

pub(crate) mod ack;
pub(crate) mod create;
pub(crate) mod list;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/sparks",
            post(create::create_spark).get(list::list_sparks),
        )
        .route("/api/v1/sparks/{id}/ack", post(ack::ack_spark))
}
