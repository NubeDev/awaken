//! Server application state wiring the committed crates into the transport.
//!
//! `AppState` holds the durable store handle (the gate's owner connection, used
//! for mutations through the WS-05 gate and for issuing WS-03 scoped read
//! sessions) plus the namespace/database a scoped session signs into. Every route
//! reads through this state; it is cloneable (an `Arc` bump on the handle) so axum
//! can share it across handlers (`rubix/STACK-DEISGN.md`, `rubix-server` row).

use std::sync::Arc;

use rubix_datasource::Registry;
use rubix_store::StoreHandle;
use tokio::sync::RwLock;

/// The datasource registry shared across handlers.
///
/// The control plane registers/removes connectors on it (`POST`/`DELETE
/// /datasources`) and the spanning query reads it, so it is a single shared,
/// mutable instance behind an async `RwLock` — not the throwaway-per-request
/// registry the list route used before. Materialised `TableProvider`s live here
/// for a connection's lifetime, so registration cost is paid once.
pub type SharedRegistry = Arc<RwLock<Registry>>;

/// Shared state injected into every request handler.
#[derive(Clone)]
pub struct AppState {
    /// The durable store boundary — the gate owner handle.
    pub store: StoreHandle,
    /// The SurrealDB namespace scoped sessions sign into.
    pub namespace: String,
    /// The SurrealDB database scoped sessions sign into.
    pub database: String,
    /// The shared datasource registry (native default + registered connectors).
    pub datasources: SharedRegistry,
}

impl AppState {
    /// Build state around an open store handle and the active namespace/database.
    ///
    /// The datasource registry starts with the native SurrealDB default; the boot
    /// path rehydrates any persisted connectors into it before serving.
    #[must_use]
    pub fn new(store: StoreHandle, namespace: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            store,
            namespace: namespace.into(),
            database: database.into(),
            datasources: Arc::new(RwLock::new(Registry::with_native_default())),
        }
    }
}
