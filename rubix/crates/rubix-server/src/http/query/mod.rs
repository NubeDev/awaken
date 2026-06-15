//! Unified query surface routes.
//!
//! One read-only query route (`rubix/docs/sessions/WS-16.md`): `POST /query`,
//! gated on the WS-04 `external-query` capability and run on the scoped session.
//! This barrel merges it into a router.

mod render;
mod run;

use axum::Router;
use axum::routing::post;

use crate::state::AppState;

use run::run_query_route;

/// The query route mounted at `/query`.
pub fn router() -> Router<AppState> {
    Router::new().route("/query", post(run_query_route))
}
