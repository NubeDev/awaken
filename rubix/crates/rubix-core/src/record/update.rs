//! UPDATE a generic record's content.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{Datetime, SurrealValue};

use crate::error::{Error, Result};
use crate::id::Id;

use super::{RECORD_TABLE, Record, RecordRow};

/// Replace the content of `record:<id>` and bump its `updated` timestamp.
///
/// `created` is preserved (a MERGE touches only `content` and `updated`), so the
/// append-only sync plane can still order writes by `updated`
/// (`rubix/docs/SCOPE.md`). Returns the updated record, or `None` if no such
/// record exists.
///
/// # Errors
/// Returns [`Error::Store`] if the write fails.
pub async fn update_record(
    db: &Surreal<Db>,
    id: &Id,
    content: serde_json::Value,
) -> Result<Option<Record>> {
    let patch = ContentPatch {
        content,
        updated: Datetime::now(),
    };
    let row: Option<RecordRow> = db
        .update((RECORD_TABLE, id.as_str()))
        .merge(patch)
        .await
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(row.map(RecordRow::into_record))
}

/// The fields a content update touches — never `id` or `created`.
#[derive(Debug, Clone, SurrealValue)]
struct ContentPatch {
    content: serde_json::Value,
    updated: Datetime,
}
