//! HTTP transport: the route table wiring the committed crates to the wire.
//!
//! One file per route, grouped by resource (`rubix/docs/FILE-LAYOUT.md`):
//! `health`, `records` (CRUD), `query`, `datasources`. Mutations cross the WS-05
//! gate; reads run on the WS-03 scoped session (contract #1). This barrel only
//! merges the resource routers into one `Router`; the WS live-query bridge and
//! the OpenAPI document are merged at the crate root (`lib.rs`).

mod datasources;
mod health;
mod query;
mod records;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

use health::health;

/// Assemble the HTTP route table over the shared [`AppState`].
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .merge(records::router())
        .merge(query::router())
        .merge(datasources::router())
}
