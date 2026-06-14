//! Server application state and router assembly.
//!
//! `AppState` holds the shared store handle that every route reads through; the
//! router is assembled here so tests can exercise routes without binding a
//! socket (`rubix/STACK-DEISGN.md`, `rubix-server` row).

use axum::Router;
use axum::routing::get;
use rubix_store::StoreHandle;

use crate::health::health;

/// Shared state injected into every request handler.
#[derive(Clone)]
pub struct AppState {
    /// The durable store boundary.
    pub store: StoreHandle,
}

impl AppState {
    /// Build state around an open store handle.
    #[must_use]
    pub fn new(store: StoreHandle) -> Self {
        Self { store }
    }
}

/// Assemble the HTTP router over the given state.
pub fn router(state: AppState) -> Router {
    Router::new().route("/health", get(health)).with_state(state)
}
