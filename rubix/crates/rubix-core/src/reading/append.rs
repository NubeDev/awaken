//! Append readings into the data plane, idempotently.
//!
//! The data-plane write (`rubix/docs/design/READINGS-TIMESERIES.md`, "Write path
//! — the data plane, never the command gate"): readings append, they do not
//! command. There is no `apply()`, no audit row, no undo capture per sample — the
//! capability decision is taken once up front by the caller (the ingest subscribe
//! or the bulk-append endpoint), and the write lands directly on the root/owner
//! handle. Because the row id is derived from `(series, at)`, a re-append or a
//! sync-replay of the same sample is an idempotent no-op: the statement is an
//! `INSERT … ON DUPLICATE KEY UPDATE` that overwrites the mutable fields and
//! leaves the original receive-time `created` untouched.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};

use super::{READING_TABLE, Reading, ReadingRow};

/// Append `readings` into the `reading` table, keyed by their deterministic ids.
///
/// One batch `INSERT`: each row keys on its `(series, at)`-derived id, so a row
/// that already exists takes the `ON DUPLICATE KEY UPDATE` branch and is
/// overwritten with identical values — a no-op effect — rather than erroring or
/// duplicating. `created` is **omitted** from the update branch so a re-append
/// preserves the original receive time (the field's `DEFAULT time::now()` only
/// stamps it on first insert). Returns the number of rows the statement wrote.
///
/// # Errors
/// Returns [`Error::Store`] if the append write fails.
pub async fn append_readings(db: &Surreal<Db>, readings: &[Reading]) -> Result<u64> {
    if readings.is_empty() {
        return Ok(0);
    }
    let rows: Vec<ReadingRow> = readings.iter().map(ReadingRow::from_reading).collect();
    let written: Vec<ReadingRow> = db
        .query(format!(
            "INSERT INTO {READING_TABLE} $rows ON DUPLICATE KEY UPDATE \
             series = $input.series, at = $input.at, value = $input.value, \
             namespace = $input.namespace, content = $input.content"
        ))
        .bind(("rows", rows))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(written.len() as u64)
}
