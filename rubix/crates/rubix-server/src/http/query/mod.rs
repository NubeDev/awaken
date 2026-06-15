//! Unified query surface routes.
//!
//! Two read-only query routes (`rubix/docs/sessions/WS-16.md`): `POST /query` for
//! one statement, and `POST /query/batch` to run a whole board in one round trip
//! (§3, `rubix/docs/design/DASHBOARDS-SCOPE.md`). Both are gated on the WS-04
//! `external-query` capability and run on the scoped session — the batch is
//! transport over the same guard, not a permission shortcut. This barrel merges
//! them into a router.

mod batch;
pub(crate) mod convert;
pub(crate) mod render;
pub(crate) mod run;

use axum::Router;
use axum::routing::post;

use crate::state::AppState;

use batch::run_batch_route;
use run::run_query_route;

/// The query routes mounted at `/query` and `/query/batch`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/query", post(run_query_route))
        .route("/query/batch", post(run_batch_route))
}
