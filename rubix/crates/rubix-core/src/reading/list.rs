//! SELECT readings — by id, in bulk, or windowed by series and time.
//!
//! Reads run on whatever connection is passed: the root store handle (all rows,
//! e.g. the seed checks and sync dedup) or a gate-issued scoped session (only the
//! principal's namespace, filtered natively by SurrealDB row-level permissions,
//! `rubix/STACK-DEISGN.md` contract #1). The windowed read is the historian's hot
//! query (`rubix/docs/design/READINGS-TIMESERIES.md`, "Read path"): a **filtered
//! SurrealQL** `WHERE series = $s AND at BETWEEN $t0 AND $t1 ORDER BY at`, which
//! the `(namespace, series, at)` index serves as a range scan — unlike a blind
//! `SELECT *` pulled to memory first.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{Datetime, RecordId};

use crate::error::{Error, Result};
use crate::id::Id;

use super::{READING_TABLE, Reading, ReadingRow};

/// The `record` table a `series` link points at (the series-defining register).
const SERIES_TABLE: &str = "record";

/// Read the reading at `reading:<id>`, or `None` if it does not exist.
///
/// Used by the sync apply path as the durable dedup check (a fresh receiver with
/// an empty in-memory seen-set must not re-insert a reading that already landed).
///
/// # Errors
/// Returns [`Error::Store`] if the read fails.
pub async fn read_reading(db: &Surreal<Db>, id: &Id) -> Result<Option<Reading>> {
    let row: Option<ReadingRow> = db
        .select((READING_TABLE, id.as_str()))
        .await
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(row.map(ReadingRow::into_reading))
}

/// Read every reading visible to `db`, ordered by measurement instant.
///
/// The visible set is decided by the connection's permissions, not this query —
/// all readings on a root handle, only the scoped namespace's on a principal
/// session. Primarily the seed/check path; charts use [`read_readings_window`].
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn list_readings(db: &Surreal<Db>) -> Result<Vec<Reading>> {
    let rows: Vec<ReadingRow> = db
        .query(format!("SELECT * FROM {READING_TABLE} ORDER BY at"))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(rows.into_iter().map(ReadingRow::into_reading).collect())
}

/// Read one series' readings whose `at` falls in `[from, to]`, ordered by `at`.
///
/// The historian's series-scoped windowed read: a filtered SurrealQL statement so
/// the `(namespace, series, at)` index can serve it as a range scan. `series` is
/// the bare register id; it is bound as a `record` link to match the stored
/// `series` field. Namespace scoping is left to the connection — on a scoped
/// session SurrealDB confines the rows to the principal's namespace, so no
/// `namespace =` clause is needed (and adding one could not widen that scope).
///
/// # Errors
/// Returns [`Error::Store`] if the query fails.
pub async fn read_readings_window(
    db: &Surreal<Db>,
    series: &str,
    from: &Datetime,
    to: &Datetime,
) -> Result<Vec<Reading>> {
    let rows: Vec<ReadingRow> = db
        .query(format!(
            "SELECT * FROM {READING_TABLE} \
             WHERE series = $series AND at >= $from AND at <= $to \
             ORDER BY at"
        ))
        .bind(("series", RecordId::new(SERIES_TABLE, series)))
        .bind(("from", *from))
        .bind(("to", *to))
        .await
        .map_err(|e| Error::Store(e.to_string()))?
        .take(0)
        .map_err(|e| Error::Store(e.to_string()))?;
    Ok(rows.into_iter().map(ReadingRow::into_reading).collect())
}
