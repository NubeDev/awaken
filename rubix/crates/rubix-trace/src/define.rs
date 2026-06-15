//! Define the append-only `trace` table and the `trace_summary` rollup surface.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Tracing"):
//! traces are append-only and bounded/sampled. The table is `SCHEMALESS` so a
//! span's free-form attributes impose no field shape, and its write permissions
//! deny mutation/deletion to every scoped principal — only the system (the
//! root/owner store handle the trace writer runs on) appends and, for retention,
//! evicts. A scoped session may read only its own tenant's spans, matching the
//! row-level scope `record` and `audit` use.
//!
//! Run once against the root handle at bootstrap. Idempotent via `OVERWRITE`.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Result, TraceError};

/// The `trace` table and its append-only permissions.
///
/// `FOR create, update, delete NONE` denies every scoped principal any write —
/// the root/owner session the trace writer and retention sweeper run on is not
/// subject to table permissions, so the system still appends and evicts while
/// principals are read-only. `FOR select WHERE namespace = $auth.namespace`
/// scopes reads to the principal's own tenant.
const TRACE_SCHEMA: &str = "\
DEFINE TABLE OVERWRITE trace SCHEMALESS\n\
  PERMISSIONS\n\
    FOR select WHERE namespace = $auth.namespace\n\
    FOR create, update, delete NONE;";

/// The `trace_summary` rollup surface and its permissions.
///
/// A Tier-B derived rollup (`rubix/docs/design/LAMINAR-BORROW.md` §5b/§7), not a
/// record: one versioned row per correlation id, upserted as spans land. Unlike
/// the append-only `trace` table this surface is *updated* in place by the
/// system, so it permits `update` only to the owner (still `NONE` for scoped
/// principals) — the upsert runs on the root/owner session, which is not subject
/// to table permissions, so principals stay read-only while the system folds.
/// Reads are scoped to the principal's own tenant, matching the `trace` table.
const SUMMARY_SCHEMA: &str = "\
DEFINE TABLE OVERWRITE trace_summary SCHEMALESS\n\
  PERMISSIONS\n\
    FOR select WHERE namespace = $auth.namespace\n\
    FOR create, update, delete NONE;";

/// Apply the append-only `trace` table definition on the root handle.
///
/// Must run on the root/owner session (the `rubix-store` handle's connection),
/// because defining table permissions is an owner action — and that same owner
/// session is what appends spans and enforces retention past the `NONE` write
/// permission.
///
/// # Errors
/// Returns [`TraceError::DefineSchema`] if the statement fails to apply.
pub async fn define_trace_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(TRACE_SCHEMA)
        .await
        .map_err(TraceError::DefineSchema)?
        .check()
        .map_err(TraceError::DefineSchema)?;
    db.query(SUMMARY_SCHEMA)
        .await
        .map_err(TraceError::DefineSchema)?
        .check()
        .map_err(TraceError::DefineSchema)?;
    Ok(())
}
