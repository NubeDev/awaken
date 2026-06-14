//! Remove the `tagged` edge between a record and a tag.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;

use crate::error::{Error, Result};
use crate::id::Id;
use crate::record::RECORD_TABLE;

use super::{TAG_TABLE, TAGGED_EDGE};

/// Remove the `tagged` edge from `record:<record_id>` to `tag:<tag_id>`.
///
/// A no-op if no such edge exists. After detach the record no longer matches a
/// tag-set query that requires this tag.
///
/// # Errors
/// Returns [`Error::Store`] if the delete fails.
pub async fn detach_tag(db: &Surreal<Db>, record_id: &Id, tag_id: &Id) -> Result<()> {
    let record = RecordId::new(RECORD_TABLE, record_id.as_str());
    let tag = RecordId::new(TAG_TABLE, tag_id.as_str());
    db.query(format!(
        "DELETE {TAGGED_EDGE} WHERE in = $record AND out = $tag"
    ))
    .bind(("record", record))
    .bind(("tag", tag))
    .await
    .map_err(|e| Error::Store(e.to_string()))?
    .check()
    .map_err(|e| Error::Store(e.to_string()))?;
    Ok(())
}
