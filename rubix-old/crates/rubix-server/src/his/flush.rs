//! Flush aged `his` rows from the SQLite hot tier into the Parquet cold tier.
//!
//! The order is read → write → delete: rows older than the retention cutoff are
//! read from SQLite, written to Parquet partitions, and only then deleted from
//! SQLite. A failure before the delete leaves the hot tier intact (the rows are
//! simply re-flushed next time); a duplicate Parquet file is harmless because
//! each flush writes a distinct, instant-named leaf and the union read dedupes
//! nothing it does not need to (a re-flushed row reappears identically).

use chrono::{DateTime, Utc};
use rubix_query::{write_partitions, HisTier};

use crate::error::FlushError;
use crate::store::Store;

/// Outcome of a flush: rows moved and partition files written.
#[derive(Debug, Clone, Copy)]
pub struct FlushReport {
    pub rows: usize,
    pub partitions: usize,
}

/// Move every `his` row older than `cutoff` into the Parquet cold tier.
///
/// Returns a zeroed report when nothing is aged. Runs blocking SQLite work off
/// the async path via the caller's executor expectations — this function is
/// async because the Parquet write is async, but the store reads/deletes are
/// quick range operations on an indexed column.
pub async fn flush_aged(
    store: &Store,
    tier: &HisTier,
    cutoff: DateTime<Utc>,
) -> Result<FlushReport, FlushError> {
    let aged = store.his_aged(cutoff)?;
    if aged.rows.is_empty() {
        return Ok(FlushReport {
            rows: 0,
            partitions: 0,
        });
    }
    let rows = aged.rows.len();
    let flush_at = Utc::now();
    let partitions = write_partitions(&tier.store(), &aged.rows, flush_at).await?;
    // Cold tier is durable; drop the hot-tier copies.
    store.his_delete_aged(cutoff)?;
    Ok(FlushReport { rows, partitions })
}
