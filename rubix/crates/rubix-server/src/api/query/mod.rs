//! Query route — wiring only.

pub(crate) mod run;

use axum::routing::post;
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/v1/query", post(run::run_query))
}
