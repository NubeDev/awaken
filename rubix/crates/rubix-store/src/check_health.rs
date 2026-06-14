//! Health probe against an open engine.
//!
//! Runs the cheapest possible round-trip so the server's `/health` route can
//! report that the store is live and answering queries.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Result, StoreError};

/// Probe the engine by executing a trivial query.
///
/// Returns `Ok(())` when the engine answers. A failed probe surfaces as
/// [`StoreError::Health`].
///
/// # Errors
/// Returns [`StoreError::Health`] if the engine does not answer the probe.
pub async fn probe(db: &Surreal<Db>) -> Result<()> {
    db.query("RETURN true").await.map_err(StoreError::Health)?;
    Ok(())
}
