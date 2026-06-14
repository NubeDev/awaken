//! Schema-init seam.
//!
//! The platform is schemaless by design — structure comes from tagging on the
//! graph, not a fixed ontology (`rubix/docs/SCOPE.md`, principle 4). This seam
//! exists so later workstreams can declare the few system tables that do need
//! definitions (audit immutability permissions, tag-edge constraints) in one
//! place. It is intentionally a no-op today.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::Result;

/// Initialise system schema on the bootstrapped connection.
///
/// No definitions exist yet; this is the wiring point for later workstreams.
///
/// # Errors
/// Returns a [`StoreError`](crate::error::StoreError) once schema statements are
/// added and one fails to apply.
pub async fn init_schema(_db: &Surreal<Db>) -> Result<()> {
    Ok(())
}
