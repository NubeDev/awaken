//! Delegate the extension's data-plane scope to WS-12 key-space scoping.
//!
//! An extension's data plane is its Zenoh subscription, and its scope is decided
//! exactly where a user's is: WS-12's [`authorize_keyspace`](rubix_ingest::authorize_keyspace)
//! (`rubix/docs/sessions/WS-13.md`, SCOPE "the data plane delegates to WS-12's
//! Zenoh key-space scoping"). This seam adds nothing — it forwards the
//! extension's [`Principal`](rubix_core::Principal) and requested key expression
//! straight through, so the *one* capability decision (the `ZenohSubscribe` grant
//! and the edge-partition inclusion check) is taken once, by the ingest
//! authority, fail closed. Keeping this a pure delegation is the point: there is
//! no second scoping path an extension could use to escape the partition a user
//! is held to.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_ingest::{AuthorizedKeySpace, authorize_keyspace};

use crate::error::{ExtError, Result};

/// Authorize `extension` to subscribe to `requested` on the data plane.
///
/// Pure delegation to WS-12 [`authorize_keyspace`](rubix_ingest::authorize_keyspace):
/// the scope is resolved once (grant check + edge-partition inclusion) and the
/// resulting [`AuthorizedKeySpace`] is the only handle the extension's subscriber
/// is opened on. An out-of-grant or out-of-partition key-space is refused here,
/// before any Zenoh session opens.
///
/// # Errors
/// Returns [`ExtError::Scope`] if WS-12 denies the subscription or the key
/// expression is malformed.
pub async fn authorize_data_scope(
    db: &Surreal<Db>,
    extension: &Principal,
    requested: &str,
) -> Result<AuthorizedKeySpace> {
    authorize_keyspace(db, extension, requested)
        .await
        .map_err(|e| ExtError::Scope(e.to_string()))
}
