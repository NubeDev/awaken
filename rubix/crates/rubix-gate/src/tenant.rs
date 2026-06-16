//! Purge every gate-owned row belonging to a tenant namespace.
//!
//! Tenant isolation in rubix is by the `namespace` **field** on each row, not by
//! a separate SurrealDB namespace (`rubix/docs/SCOPE.md`, "Edge and cloud
//! profiles"; the row-level read scope keys off `namespace = $auth.namespace`).
//! Dropping a tenant therefore means deleting every row tagged with that
//! namespace from the gate-owned tables — records, principals, and grants — and
//! is the irreversible operation `rubix/docs/design/ADMIN-API.md` (Surface 3,
//! Open item 1) gates behind a root/system principal.
//!
//! This is an owner action on the root handle: the tables it clears (`principal`,
//! `grant`) have no scoped-session write path, and `record` is gate-owned. One
//! `delete` audit row is appended for the namespace as a whole — the registry
//! record (server-side) is deleted separately by the caller.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{CorrelationId, Id, Principal};

use crate::audit::{AuditRecord, append_audit};
use crate::command::CapturedChange;
use crate::error::{GateError, Result};

/// The gate-owned tables a tenant's rows live in, all keyed by a `namespace`
/// field. `record` holds data + definitions; `principal`/`grant` hold identity.
const TENANT_OWNED_TABLES: [&str; 3] = ["record", "principal", "grant"];

/// Delete every gate-owned row in `namespace`, appending one audit row.
///
/// `actor` is the root/system principal performing the purge (the audit subject).
/// Returns the total number of rows deleted across the gate-owned tables, so the
/// caller can report what was removed rather than silently dropping data.
///
/// # Errors
/// Returns [`GateError::CommandApply`] if a delete fails or
/// [`GateError::AuditWrite`] if the audit append fails.
pub async fn purge_namespace(
    db: &Surreal<Db>,
    actor: &Principal,
    namespace: &str,
) -> Result<usize> {
    let mut deleted = 0usize;
    for table in TENANT_OWNED_TABLES {
        let mut response = db
            .query(format!(
                "DELETE FROM {table} WHERE namespace = $namespace RETURN BEFORE"
            ))
            .bind(("namespace", namespace.to_owned()))
            .await
            .map_err(GateError::CommandApply)?;
        let rows: Vec<serde_json::Value> = response.take(0).map_err(GateError::CommandApply)?;
        deleted += rows.len();
    }

    let captured = CapturedChange {
        before: Some(serde_json::json!({ "namespace": namespace, "rows_deleted": deleted })),
        after: None,
    };
    let correlation_id = CorrelationId::mint();
    let target = Id::from_raw(format!("tenant:{namespace}"));
    let record = AuditRecord::project(actor, "delete", &target, &captured, &correlation_id);
    append_audit(db, &record).await?;
    Ok(deleted)
}
