//! Server application state wiring the committed crates into the transport.
//!
//! `AppState` holds the durable store handle (the gate's owner connection, used
//! for mutations through the WS-05 gate and for issuing WS-03 scoped read
//! sessions) plus the namespace/database a scoped session signs into. Every route
//! reads through this state; it is cloneable (an `Arc` bump on the handle) so axum
//! can share it across handlers (`rubix/STACK-DEISGN.md`, `rubix-server` row).

use rubix_store::StoreHandle;

/// Shared state injected into every request handler.
#[derive(Clone)]
pub struct AppState {
    /// The durable store boundary — the gate owner handle.
    pub store: StoreHandle,
    /// The SurrealDB namespace scoped sessions sign into.
    pub namespace: String,
    /// The SurrealDB database scoped sessions sign into.
    pub database: String,
}

impl AppState {
    /// Build state around an open store handle and the active namespace/database.
    #[must_use]
    pub fn new(store: StoreHandle, namespace: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            store,
            namespace: namespace.into(),
            database: database.into(),
        }
    }
}
