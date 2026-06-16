//! Retention sweep — age raw readings out of one edge partition.
//!
//! Retention on the data plane is a **cheap range delete**, not a scan
//! (`rubix/docs/design/READINGS-TIMESERIES.md`, "Retention is a cheap sweep"):
//! append-only writes plus the `reading_ns_at` index (`FIELDS namespace, at`) make
//! `DELETE FROM reading WHERE namespace = $ns AND at < $cutoff` a bounded range
//! delete on `(namespace, at)`. This is the relief valve for SCOPE's single-engine
//! write/pub-sub concentration bet on the readings axis.
//!
//! This module is the **primitive only**. The retention *policy surface* — per
//! the design's open question, whether the cutoff is chosen per-series vs.
//! per-namespace vs. global, the TTL values, and the schedule that drives it — is
//! deliberately not wired here; that decision belongs to a caller (a sweep job or
//! an admin action), not to the delete itself. Per-namespace is the natural unit
//! because readings are edge-partitioned by `namespace`, so this primitive sweeps
//! one partition at a time and a caller fans out over the partitions it owns.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::Datetime;

use crate::error::{Error, Result};

use super::{READING_TABLE, ReadingRow};

/// Delete every reading in `namespace` whose `at` is strictly before `cutoff`,
/// returning the number of rows removed.
///
/// The cutoff is on **measurement** time (`at`), never receive time (`created`),
/// matching how the read path buckets — a sample is retained by when the world
/// produced it, not when we happened to persist it. The bound is exclusive
/// (`at < cutoff`), so a cutoff equal to a sample's instant keeps that sample.
/// `RETURN BEFORE` yields the deleted rows so the count is exact rather than
/// inferred. Runs on whatever handle is passed — the root/owner handle for a
/// maintenance sweep, off the command gate, exactly like the append path.
///
/// # Errors
/// Returns [`Error::Store`] if the delete fails.
pub async fn sweep_readings_before(
    db: &Surreal<Db>,
    namespace: &str,
    cutoff: &Datetime,
) -> Result<u64> {
    let deleted: Vec<ReadingRow> = db
        .query(format!(
            "DELETE FROM {READING_TABLE} \
             WHERE namespace = $namespace AND at < $cutoff \
             RETURN BEFORE"
        ))
        .bind(("namespace", namespace.to_owned()))
        .bind(("cutoff", *cutoff))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(deleted.len() as u64)
}
