//! SELECT readable records, optionally narrowed by collection kind and tag set.
//!
//! Returns every record the connection is permitted to read. On the root store
//! handle that is all records; on a gate-issued scoped session it is only the
//! principal's namespace data, because SurrealDB row-level permissions filter
//! the result natively (`rubix/STACK-DEISGN.md`, contracts #1/#2). A `kind`/`tag`
//! filter narrows *on top of* that scope (`BACKEND-COLLECTIONS.md`,
//! "List/realtime filtering by collection") — it can never widen it, since the
//! query still runs on the same scoped session, so it stays inside contract #1.

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
    list_records_filtered(db, None, &[]).await
}

/// Read the visible records narrowed by collection `kind` and/or a `tag` set.
///
/// `kind` matches `content.kind` (the collection a record belongs to); `tags`
/// matches by **tag name** over the `record→tagged→tag` graph, Haystack-style —
/// a record must carry *all* requested tags (set intersection). Either filter is
/// optional: `None`/empty omits that clause, so `list_records_filtered(db, None,
/// &[])` is exactly [`list_records`]. Both narrow the session's already-scoped
/// view; neither can widen it.
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn list_records_filtered(
    db: &Surreal<Db>,
    kind: Option<&str>,
    tags: &[String],
) -> Result<Vec<Record>> {
    let mut clauses: Vec<&str> = Vec::new();
    if kind.is_some() {
        clauses.push("content.kind = $kind");
    }
    if !tags.is_empty() {
        clauses.push("array::len(array::intersect(->tagged.out.name, $tags)) = $wanted");
    }
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };

    let mut query = db.query(format!("SELECT * FROM {RECORD_TABLE}{where_clause}"));
    if let Some(kind) = kind {
        query = query.bind(("kind", kind.to_owned()));
    }
    if !tags.is_empty() {
        query = query
            .bind(("tags", tags.to_vec()))
            .bind(("wanted", tags.len() as i64));
    }

    let rows: Vec<RecordRow> = query
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(rows.into_iter().map(RecordRow::into_record).collect())
}
