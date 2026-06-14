//! CREATE a generic record.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};

use super::{RECORD_TABLE, Record, RecordRow};

/// Persist `record` into the `record` table, keyed by its own id.
///
/// Returns the stored record as SurrealDB decoded it (the round-trip confirms
/// the write landed). Persisting through an explicit `record:<id>` thing keeps
/// ids edge-mintable without coordination (`rubix/docs/SCOPE.md`).
///
/// # Errors
/// Returns [`Error::Store`] if the write fails or the row is not returned.
pub async fn create_record(db: &Surreal<Db>, record: &Record) -> Result<Record> {
    let created: Option<RecordRow> = db
        .create((RECORD_TABLE, record.id.as_str()))
        .content(RecordRow::from_record(record))
        .await
        .map_err(|e| Error::Store(e.to_string()))?;
    created
        .map(RecordRow::into_record)
        .ok_or_else(|| Error::Store("record create returned no row".to_owned()))
}
