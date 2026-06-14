//! SELECT all readable records.
//!
//! Returns every record the connection is permitted to read. On the root store
//! handle that is all records; on a gate-issued scoped session it is only the
//! principal's namespace data, because SurrealDB row-level permissions filter
//! the result natively (`rubix/STACK-DEISGN.md`, contracts #1/#2). The verb
//! itself imposes no filter — the engine owns the scope.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};

use super::{RECORD_TABLE, Record, RecordRow};

/// Read every record visible to `db`.
///
/// The visible set is decided by the session's permissions, not by this query —
/// the same `SELECT` returns all records on a root session and only the scoped
/// namespace's records on a principal session.
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn list_records(db: &Surreal<Db>) -> Result<Vec<Record>> {
    let rows: Vec<RecordRow> = db
        .query(format!("SELECT * FROM {RECORD_TABLE}"))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(rows.into_iter().map(RecordRow::into_record).collect())
}
