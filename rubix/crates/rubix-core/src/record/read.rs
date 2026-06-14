//! SELECT a generic record by id.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};
use crate::id::Id;

use super::{RECORD_TABLE, Record, RecordRow};

/// Read the record at `record:<id>`, or `None` if it does not exist.
///
/// # Errors
/// Returns [`Error::Store`] if the read fails.
pub async fn read_record(db: &Surreal<Db>, id: &Id) -> Result<Option<Record>> {
    let row: Option<RecordRow> = db
        .select((RECORD_TABLE, id.as_str()))
        .await
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(row.map(RecordRow::into_record))
}
