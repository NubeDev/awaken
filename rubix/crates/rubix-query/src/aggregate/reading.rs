//! Extract a numeric time-series from the `reading` data plane, scoped.
//!
//! A rollup needs `(timestamp, value)` samples; this reads them from the
//! `reading` table through the principal's scoped session — so the samples are
//! exactly the rows SurrealDB permissions admit (contract #1) — and projects
//! each row to a [`Sample`] of its **measurement instant `at`** and its typed
//! numeric `value` column. Unlike [`super::series`], which buckets generic
//! `record` rows on `created` (write time) and reads the value out of free-form
//! `content`, this reader buckets on `at` and reads `value` directly: that is
//! the root fix for the trend-collapse bug (`rubix/docs/design/READINGS-TIMESERIES.md`,
//! "Read path"). A row missing `at` or carrying no numeric `value` is skipped,
//! not defaulted: a missing reading must not be silently aggregated as zero,
//! which would corrupt the value a rule decides on.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{QueryError, Result};

use super::rollup::Sample;

/// Read `(at, value)` samples from the `reading` table on `session`.
///
/// Timestamps come from each row's `at` column — the **measurement** instant,
/// not the `created` write time — parsed to epoch microseconds; values come from
/// the typed numeric `value` column directly (not `content.<field>`). Rows
/// lacking either are skipped. The scoped session bounds which rows are read.
///
/// # Errors
/// Returns [`QueryError::Scan`] if the SurrealDB read fails.
pub async fn read_reading_series(session: &Surreal<Db>) -> Result<Vec<Sample>> {
    let mut response = session
        .query("SELECT * FROM reading")
        .await
        .map_err(|e| QueryError::Scan(e.to_string()))?;
    let rows: Vec<serde_json::Value> = response
        .take(0)
        .map_err(|e| QueryError::Scan(e.to_string()))?;

    Ok(rows.iter().filter_map(sample_of).collect())
}

/// Project one reading row into a [`Sample`], or `None` if it carries no usable
/// `at`/`value`.
///
/// Bucketing time is the `at` column (measurement instant), so a back-dated
/// sample (`at` long ago, `created` now) lands in the bucket its `at` selects —
/// the trend-collapse fix. The value is the typed numeric `value` column, read
/// directly rather than reached out of `content`.
fn sample_of(row: &serde_json::Value) -> Option<Sample> {
    let at_micros = row
        .get("at")
        .and_then(serde_json::Value::as_str)
        .and_then(crate::provider::parse_created_micros)?;
    let value = row.get("value").and_then(serde_json::Value::as_f64)?;
    Some(Sample { at_micros, value })
}

#[cfg(test)]
mod tests {
    use super::sample_of;

    #[test]
    fn a_sample_buckets_on_at_not_created() {
        // `at` is the measurement instant (back-dated); `created` is write time
        // (now). The sample must take its instant from `at`, closing the
        // trend-collapse bug.
        let row = serde_json::json!({
            "at": "2026-06-14T10:00:00Z",
            "created": "2026-06-16T00:00:00Z",
            "value": 5.0
        });
        let sample = sample_of(&row).unwrap();
        // 2026-06-14T10:00:00Z in epoch micros — the `at` value, not `created`.
        let at_micros =
            crate::provider::parse_created_micros("2026-06-14T10:00:00Z").unwrap();
        let created_micros =
            crate::provider::parse_created_micros("2026-06-16T00:00:00Z").unwrap();
        assert_eq!(sample.at_micros, at_micros);
        assert_ne!(sample.at_micros, created_micros);
        assert!((sample.value - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_row_missing_value_is_skipped() {
        let row = serde_json::json!({ "at": "2026-06-14T10:00:00Z" });
        assert_eq!(sample_of(&row), None);
    }

    #[test]
    fn a_non_numeric_value_is_skipped() {
        let row = serde_json::json!({
            "at": "2026-06-14T10:00:00Z",
            "value": "warm"
        });
        assert_eq!(sample_of(&row), None);
    }

    #[test]
    fn a_row_missing_at_is_skipped() {
        let row = serde_json::json!({ "value": 5.0 });
        assert_eq!(sample_of(&row), None);
    }
}
