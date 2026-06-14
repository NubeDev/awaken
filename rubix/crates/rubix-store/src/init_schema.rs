//! Schema-init seam.
//!
//! The platform is schemaless by design — structure comes from tagging on the
//! graph, not a fixed ontology (`rubix/docs/SCOPE.md`, principle 4). This seam
//! declares the few tables that must exist up front so reads against an empty
//! database return no rows rather than erroring on a missing table. The tables
//! are `SCHEMALESS` (and the tag link is `TYPE RELATION`) so they impose no
//! field shape — the generic record's content stays free-form.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Result, StoreError};

/// The idempotent table definitions every namespace needs before first read.
///
/// Defined here, not in `rubix-core`, because table existence is a store-boundary
/// concern; `rubix-core` only emits the record/tag verbs that operate over them.
const SCHEMA: &str = "\
DEFINE TABLE IF NOT EXISTS record SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS tag SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS tagged TYPE RELATION SCHEMALESS;";

/// Initialise system schema on the bootstrapped connection.
///
/// Idempotent: re-running against an already-initialised database is a no-op via
/// `IF NOT EXISTS`. Later workstreams append their own definitions here.
///
/// # Errors
/// Returns [`StoreError::Bootstrap`] if a schema statement fails to apply.
pub async fn init_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(SCHEMA)
        .await
        .map_err(StoreError::Bootstrap)?
        .check()
        .map_err(StoreError::Bootstrap)?;
    Ok(())
}
