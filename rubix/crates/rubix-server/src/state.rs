//! Server application state wiring the committed crates into the transport.
//!
//! `AppState` holds the durable store handle (the gate's owner connection, used
//! for mutations through the WS-05 gate and for issuing WS-03 scoped read
//! sessions) plus the namespace/database a scoped session signs into. Every route
//! reads through this state; it is cloneable (an `Arc` bump on the handle) so axum
//! can share it across handlers (`rubix/STACK-DEISGN.md`, `rubix-server` row).

use std::sync::Arc;
use std::time::Duration;

use rubix_blob::{BlobStore, LocalFsBlobStore};
use rubix_datasource::Registry;
use rubix_ext::runtime::ExtensionRuntime;
use rubix_query::ContextCache;
use rubix_store::StoreHandle;
use tokio::sync::RwLock;

use crate::jobs::JobRegistry;
use crate::profile::Profile;

/// The soft deadline a Tier-1 bulk op runs against before promoting to a Tier-2
/// job (`rubix/docs/design/BULK-AND-JOBS.md`, OQ1: "start conservative ~10s").
///
/// Measured per request; an op that has not finished all items by this point
/// promotes the remainder to a background job, returning `202` with the
/// already-committed statuses. Tests override this (e.g. to zero) to force
/// promotion deterministically.
pub const DEFAULT_BULK_DEADLINE: Duration = Duration::from_secs(10);

/// The datasource registry shared across handlers.
///
/// The control plane registers/removes connectors on it (`POST`/`DELETE
/// /datasources`) and the spanning query reads it, so it is a single shared,
/// mutable instance behind an async `RwLock` — not the throwaway-per-request
/// registry the list route used before. Materialised `TableProvider`s live here
/// for a connection's lifetime, so registration cost is paid once.
pub type SharedRegistry = Arc<RwLock<Registry>>;

/// The per-principal scanned-context cache shared across handlers (§4a).
///
/// One instance per server, behind an `Arc`, so a board tick and different SQL on
/// the same tables reuse a principal's scan instead of rescanning every canonical
/// table. The cache locks internally, so it needs no outer `RwLock`.
pub type SharedContextCache = Arc<ContextCache>;

/// The blob store shared across handlers.
///
/// A trait object so the backend is pluggable: the local-filesystem store on edge,
/// an object store on cloud (behind the `cloud` feature). Behind an `Arc` because
/// the file routes hold it for a request's lifetime and it is cloned into state.
pub type SharedBlobStore = Arc<dyn BlobStore>;

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
    /// The per-principal scanned-context cache (§4a).
    pub context_cache: SharedContextCache,
    /// The binary blob store backing `file` fields (`POST`/`GET /files`).
    pub blobs: SharedBlobStore,
    /// The deployment profile this server booted into (WS-14). The gate reads its
    /// namespace strategy to resolve a request's tenant; routes read its
    /// `auth_required`/`sync_enabled` defaults from one place.
    pub profile: Profile,
    /// The in-memory long-running job registry (`BULK-AND-JOBS.md`). Bulk ops that
    /// promote to a background job register here; the WS job channel and the status
    /// poll read from it. Cloneable (an `Arc` bump), so it is shared across handlers
    /// and survives an `AppState` clone as the *same* registry.
    pub jobs: JobRegistry,
    /// The Tier-1 bulk soft deadline before a bulk op promotes to a Tier-2 job.
    pub bulk_deadline: Duration,
    /// The extension runtime: the live supervisor map + metrics registry the
    /// `/extensions*` admin surface reads and the gated lifecycle route drives
    /// (`rubix/docs/design/EXTENSION-RUNTIME.md`). Cloneable (an `Arc` bump per
    /// inner registry), so an `AppState` clone shares the *same* live runtime —
    /// the boot reconciler and every request handler see one world.
    pub extensions: ExtensionRuntime,
}

impl AppState {
    /// Build state around an open store handle and the active namespace/database,
    /// defaulting to the edge profile.
    ///
    /// The datasource registry starts with the native SurrealDB default; the boot
    /// path rehydrates any persisted connectors into it before serving. The binary
    /// uses [`AppState::with_profile`] to thread the selected deployment profile;
    /// this constructor keeps the single-namespace edge default for callers (and
    /// tests) that do not select one.
    #[must_use]
    pub fn new(
        store: StoreHandle,
        namespace: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self::with_profile(
            store,
            namespace,
            database,
            crate::profile::default_profile(),
        )
    }

    /// Build state around an open store handle, namespace/database, and a selected
    /// deployment [`Profile`] (WS-14).
    ///
    /// The boot path resolves the profile from `RUBIX_PROFILE`
    /// ([`profile::from_env`](crate::profile::from_env)) and threads it here so
    /// every handler reads the same per-profile defaults.
    #[must_use]
    pub fn with_profile(
        store: StoreHandle,
        namespace: impl Into<String>,
        database: impl Into<String>,
        profile: Profile,
    ) -> Self {
        Self {
            store,
            namespace: namespace.into(),
            database: database.into(),
            datasources: Arc::new(RwLock::new(Registry::with_native_default())),
            context_cache: Arc::new(ContextCache::default()),
            blobs: default_blob_store(),
            profile,
            jobs: JobRegistry::default(),
            bulk_deadline: DEFAULT_BULK_DEADLINE,
            extensions: ExtensionRuntime::new(),
        }
    }
}

/// The default blob store: a local-filesystem store under `RUBIX_DATA_DIR/blobs`,
/// or — when that env is unset (tests, ephemeral runs) — an isolated temp
/// directory unique to this process state.
///
/// The binary overrides `state.blobs` with a store rooted at its configured data
/// directory; this default keeps every constructor (and every test) working
/// without threading a path through, while never sharing a root between unrelated
/// runs.
fn default_blob_store() -> SharedBlobStore {
    let root = match std::env::var_os("RUBIX_DATA_DIR") {
        Some(dir) => std::path::PathBuf::from(dir).join("blobs"),
        None => std::env::temp_dir().join(format!("rubix-blobs-{}", uuid::Uuid::new_v4())),
    };
    Arc::new(LocalFsBlobStore::open(root))
}
