//! Define the append-only, immutable `audit` table permissions.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Audit log"):
//! audit rows are immutable — "no UPDATE/DELETE grant to any principal but the
//! system". This statement encodes that in the engine, not in app code:
//!
//! - `FOR select WHERE namespace = $auth.namespace` — a scoped session reads
//!   only its own tenant's audit rows (same row-level scope as `record`);
//! - `FOR create, update, delete NONE` — no scoped-session principal may write,
//!   update, or delete an audit row. The gate appends rows on the root/owner
//!   store handle, whose owner session is not subject to table permissions, so
//!   the *system* still writes while every principal is denied mutation.
//!
//! Run once against the root handle at bootstrap, after `define_gate_schema`.
//! Idempotent via `IF NOT EXISTS` on the table and `OVERWRITE` on the permission
//! re-application.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{GateError, Result};

/// The audit table and its immutability permissions.
///
/// The `FOR create, update, delete NONE` clause is the load-bearing line: the
/// engine itself refuses any principal's attempt to write, change, or remove an
/// audit row, so immutability does not depend on application discipline.
const AUDIT_SCHEMA: &str = "\
DEFINE TABLE OVERWRITE audit SCHEMALESS\n\
  PERMISSIONS\n\
    FOR select WHERE namespace = $auth.namespace\n\
    FOR create, update, delete NONE;";

/// Apply the append-only, immutable `audit` table permissions on the root handle.
///
/// Must run on the root/owner session (the `rubix-store` handle's connection),
/// because defining table permissions is an owner action — and because that same
/// owner session is what the gate uses to append audit rows past the `NONE`
/// write permission.
///
/// # Errors
/// Returns [`GateError::DefineSchema`] if the statement fails to apply.
pub async fn define_audit_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(AUDIT_SCHEMA)
        .await
        .map_err(GateError::DefineSchema)?
        .check()
        .map_err(GateError::DefineSchema)?;
    Ok(())
}
