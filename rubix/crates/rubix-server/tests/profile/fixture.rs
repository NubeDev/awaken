//! Boot an `AppState` over an in-memory store under a selected profile.
//!
//! The WS-14 boot tests assert tenant resolution per profile on kv-mem: edge
//! resolves every request to the one configured namespace, cloud resolves a
//! namespace per tenant. This fixture opens the store and threads the selected
//! [`Profile`] into `AppState` exactly as the binary does, so the assertions run
//! against the same construction path the server boots through.

use rubix_core::RuntimeConfig;
use rubix_server::profile::Profile;
use rubix_server::AppState;
use rubix_store::StoreHandle;

/// Namespace the profile boot tests configure as the single/base namespace.
pub const NS: &str = "rubix";

/// Open an in-memory store and build `AppState` under `profile`.
///
/// `database` keeps each test's kv-mem instance isolated.
pub async fn boot(database: &str, profile: Profile) -> AppState {
    let cfg = RuntimeConfig::in_memory(NS, database);
    let store = StoreHandle::open(&cfg).await.expect("open in-memory store");
    AppState::with_profile(store, NS, database, profile)
}
