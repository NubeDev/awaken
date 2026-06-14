//! Decode a generic record from a SurrealDB value.
//!
//! A live-query notification (the data-change plane in `rubix-bus`) delivers the
//! changed record as a SurrealDB [`Value`], not a typed row. This verb maps that
//! value back through the persisted [`RecordRow`] into a domain [`Record`], so
//! the data-change plane reuses the one record decode path rather than
//! duplicating the row shape (`rubix/docs/FILE-LAYOUT.md`, dedup).

use surrealdb::types::{SurrealValue, Value};

use crate::error::{Error, Result};

use super::{Record, RecordRow};

/// Decode a SurrealDB `value` (e.g. a live-query notification payload) into a
/// domain [`Record`].
///
/// # Errors
/// Returns [`Error::Store`] if `value` is not the shape of a persisted record.
pub fn decode_record(value: Value) -> Result<Record> {
    let row = RecordRow::from_value(value).map_err(|e| Error::Store(e.to_string()))?;
    Ok(row.into_record())
}
