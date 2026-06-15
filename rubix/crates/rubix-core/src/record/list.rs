//! SELECT readable records, optionally narrowed by collection kind and tag set.
//!
//! Returns every record the connection is permitted to read. On the root store
//! handle that is all records; on a gate-issued scoped session it is only the
//! principal's namespace data, because SurrealDB row-level permissions filter
//! the result natively (`rubix/STACK-DEISGN.md`, contracts #1/#2). A `kind`/`tag`
//! filter narrows *on top of* that scope (`BACKEND-COLLECTIONS.md`,
//! "List/realtime filtering by collection") — it can never widen it, since the
//! query still runs on the same scoped session, so it stays inside contract #1.

use std::collections::HashMap;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::SurrealValue;

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

/// A record's id paired with the names of the tags it carries.
///
/// Tags are graph edges (`record→tagged→tag`), not record fields, so this is a
/// **read-only projection** at the read boundary — never part of the durable
/// [`Record`], which a write persists. A consumer joins this onto a record listing
/// to surface tags on the wire without coupling the write path to the tag graph.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub struct RecordTags {
    /// The record's string id (the key after `record:`).
    pub id: String,
    /// The names of the tags this record carries, in graph order.
    pub tags: Vec<String>,
}

/// Project the tag names of every record visible to `db`, keyed by record id.
///
/// Runs on the same session as [`list_records`], so it sees exactly the records
/// the principal may read — the projection narrows with the scope, never widens
/// it. Returned as a map for an O(1) join onto a record listing.
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn list_record_tags(db: &Surreal<Db>) -> Result<HashMap<String, Vec<String>>> {
    let rows: Vec<RecordTags> = db
        .query(format!(
            "SELECT meta::id(id) AS id, ->tagged->tag.name AS tags FROM {RECORD_TABLE}"
        ))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(rows.into_iter().map(|r| (r.id, r.tags)).collect())
}
