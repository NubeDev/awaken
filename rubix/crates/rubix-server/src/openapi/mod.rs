//! OpenAPI 3.1 surface.
//!
//! Serves the utoipa-built document at `/api-docs/openapi.json`
//! (`rubix/docs/sessions/WS-16.md`). [`document`](document::document) assembles
//! the definition; [`paths`] carries the per-route annotations; [`serve`] is the
//! route. This barrel exposes the document builder and the router.

mod document;
mod paths;
mod serve;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

use serve::serve_openapi;

pub use document::document;

/// The OpenAPI route mounted at `/api-docs/openapi.json`.
pub fn router() -> Router<AppState> {
    Router::new().route("/api-docs/openapi.json", get(serve_openapi))
}
