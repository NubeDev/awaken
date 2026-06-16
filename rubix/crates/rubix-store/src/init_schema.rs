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
///
/// The `reading` table is the time-series data plane
/// (`rubix/docs/design/READINGS-TIMESERIES.md`): high-volume, append-only, lean
/// rows of `{ series, at, value }` + the `namespace` edge partition. It stays
/// `SCHEMALESS` (principle 4 — extra fields land free-form in `content`), but the
/// three columns a time-series read lives or dies on are typed, and the hot read
/// ("this namespace, this series, this window") is served by the namespace-first
/// `(namespace, series, at)` index as a range scan. The row-level `PERMISSIONS`
/// that scope `reading` to a session's namespace are overwritten in `rubix-gate`'s
/// `define_gate_schema` — the two land together or the table is unsafe.
const SCHEMA: &str = "\
DEFINE TABLE IF NOT EXISTS record SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS tag SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS tagged TYPE RELATION SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS reading SCHEMALESS;\n\
DEFINE FIELD IF NOT EXISTS series ON reading TYPE record;\n\
DEFINE FIELD IF NOT EXISTS at ON reading TYPE datetime;\n\
DEFINE FIELD IF NOT EXISTS value ON reading TYPE number;\n\
DEFINE FIELD IF NOT EXISTS namespace ON reading TYPE string;\n\
DEFINE FIELD IF NOT EXISTS created ON reading TYPE datetime DEFAULT time::now();\n\
DEFINE INDEX IF NOT EXISTS reading_ns_series_at ON reading FIELDS namespace, series, at;\n\
DEFINE INDEX IF NOT EXISTS reading_ns_at ON reading FIELDS namespace, at;";

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
