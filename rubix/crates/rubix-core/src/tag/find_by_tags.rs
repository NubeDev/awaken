//! Graph traversal: records carrying a whole tag set (Haystack-style).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;

use crate::error::{Error, Result};
use crate::id::Id;
use crate::record::{RECORD_TABLE, Record, RecordRow};

use super::TAG_TABLE;

/// Find every record connected to **all** of `tag_ids` (set intersection).
///
/// This is the Haystack model: a record matches only if it carries the full
/// requested tag set; partial matches are excluded. An empty `tag_ids` matches
/// nothing, since "carries all of no tags" has no useful meaning for a filter.
///
/// The intersection is computed in SurrealDB over the `record→tagged→tag` graph
/// (contract #6: graph traversal in the one engine), comparing the record's tag
/// things against the requested set and requiring the overlap to cover every
/// requested tag.
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn find_records_by_tags(db: &Surreal<Db>, tag_ids: &[Id]) -> Result<Vec<Record>> {
    if tag_ids.is_empty() {
        return Ok(Vec::new());
    }
    let things: Vec<RecordId> = tag_ids
        .iter()
        .map(|id| RecordId::new(TAG_TABLE, id.as_str()))
        .collect();
    let wanted = things.len();
    let mut response = db
        .query(format!(
            "SELECT * FROM {RECORD_TABLE} \
             WHERE array::len(array::intersect(->tagged.out, $tags)) = $wanted"
        ))
        .bind(("tags", things))
        .bind(("wanted", wanted as i64))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .check()
        .map_err(|e| Error::Store(e.to_string()))?;
    let rows: Vec<RecordRow> = response.take(0).map_err(|e| Error::Store(e.to_string()))?;
    Ok(rows.into_iter().map(RecordRow::into_record).collect())
}
