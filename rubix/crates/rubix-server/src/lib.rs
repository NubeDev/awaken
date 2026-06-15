//! Rubix server: the transport that wires every committed crate to the wire.
//!
//! The integration layer (`rubix/STACK-DEISGN.md`, `rubix-server` row;
//! `rubix/docs/sessions/WS-16.md`): axum HTTP routes (mutations through the WS-05
//! gate, reads on the WS-03 scoped session), a WebSocket bridge over the WS-07
//! live-query feed, the utoipa OpenAPI 3.1 document, and the `rubix-prefs` display
//! layer. The library assembles the [`router`] so integration tests exercise the
//! routes without binding a socket; the binary (`main.rs`) opens the store, builds
//! the [`AppState`], and serves it.

mod auth;
pub mod datasources;
mod dto;
mod error;
mod http;
mod openapi;
pub mod profile;
pub mod seed;
mod state;
mod ws;

use axum::Router;

pub use datasources::{define_datasource_schema, rehydrate};
pub use error::{ApiError, ApiResult};
pub use openapi::document as openapi_document;
pub use profile::{NamespaceStrategy, Profile, ProfileError};
pub use seed::seed_dev;
pub use state::AppState;

/// Assemble the full transport router over the given state.
///
/// Merges the HTTP route table, the WebSocket live-query bridge, and the OpenAPI
/// document route, then injects the shared [`AppState`].
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(http::router())
        .merge(ws::router())
        .merge(openapi::router())
        .with_state(state)
}
