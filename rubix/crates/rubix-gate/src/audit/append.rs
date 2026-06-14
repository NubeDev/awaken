//! Append an audit row — the only write path into the immutable audit table.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`): audit is append-only and immutable.
//! This verb writes one [`AuditRecord`] to the per-namespace `audit` table on
//! the root/owner store handle — the only session permitted past the table's
//! `FOR create … NONE` permission (see [`permit`](super::permit)). Each row is
//! keyed by a fresh id, so appends never collide and a row is never overwritten.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{Datetime, RecordId, SurrealValue};

use rubix_core::Id;

use crate::error::{GateError, Result};

use super::record::AuditRecord;

/// The table audit rows live in.
const AUDIT_TABLE: &str = "audit";

/// Append `audit` to the immutable audit table on the root handle.
///
/// The row is keyed by a fresh id and stamped with the current time. Runs on the
/// owner session because the table denies `create` to every scoped principal —
/// only the system appends (`rubix/docs/SCOPE.md`, "Audit log").
///
/// # Errors
/// Returns [`GateError::AuditWrite`] if the append fails.
pub(crate) async fn append_audit(db: &Surreal<Db>, audit: &AuditRecord) -> Result<()> {
    let key = Id::new();
    let row = AuditRow::from_record(audit, &key);
    let _: Option<AuditRow> = db
        .create((AUDIT_TABLE, key.as_str()))
        .content(row)
        .await
        .map_err(GateError::AuditWrite)?;
    Ok(())
}

/// SurrealDB-facing audit row: the reserved `id` thing plus the audit fields.
///
/// `before`/`after` are stored as free-form JSON summaries; `at` is the append
/// timestamp. The correlation id is stored as its string form so it threads to
/// the trace and undo planes by value.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
struct AuditRow {
    id: RecordId,
    subject: String,
    namespace: String,
    action: String,
    target: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    correlation_id: String,
    at: Datetime,
}

impl AuditRow {
    fn from_record(audit: &AuditRecord, key: &Id) -> Self {
        Self {
            id: RecordId::new(AUDIT_TABLE, key.as_str()),
            subject: audit.subject.clone(),
            namespace: audit.namespace.clone(),
            action: audit.action.clone(),
            target: audit.target.clone(),
            before: audit.before.clone(),
            after: audit.after.clone(),
            correlation_id: audit.correlation_id.to_string(),
            at: Datetime::now(),
        }
    }
}
