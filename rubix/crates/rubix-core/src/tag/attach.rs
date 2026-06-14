//! RELATE a record to a tag (`record→tagged→tag`).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::RecordId;

use crate::error::{Error, Result};
use crate::id::Id;
use crate::record::RECORD_TABLE;

use super::{TAG_TABLE, TAGGED_EDGE};

/// Connect `record:<record_id>` to `tag:<tag_id>` with a `tagged` edge.
///
/// Idempotent: any existing edge between the pair is removed first, so calling
/// `attach` twice never produces a duplicate edge that would skew tag-set
/// intersection counts in [`find_records_by_tags`](super::find_records_by_tags).
///
/// # Errors
/// Returns [`Error::Store`] if the relate fails.
pub async fn attach_tag(db: &Surreal<Db>, record_id: &Id, tag_id: &Id) -> Result<()> {
    let record = RecordId::new(RECORD_TABLE, record_id.as_str());
    let tag = RecordId::new(TAG_TABLE, tag_id.as_str());
    db.query(format!(
        "DELETE {TAGGED_EDGE} WHERE in = $record AND out = $tag"
    ))
    .query(format!("RELATE $record->{TAGGED_EDGE}->$tag"))
    .bind(("record", record))
    .bind(("tag", tag))
    .await
    .map_err(|e| Error::Store(e.to_string()))?
    .check()
    .map_err(|e| Error::Store(e.to_string()))?;
    Ok(())
}
