//! Extract a numeric time-series from a canonical table, scoped.
//!
//! A rollup needs `(timestamp, value)` samples; this reads them from a canonical
//! table through the principal's scoped session — so the samples are exactly the
//! rows SurrealDB permissions admit (contract #1) — and projects each row to a
//! [`Sample`] of its `created` instant and a numeric field read from the row's
//! free-form `content`. A row whose `content` field is absent or non-numeric is
//! skipped, not defaulted: a missing reading must not be silently aggregated as
//! zero, which would corrupt the value a rule decides on.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{QueryError, Result};
use crate::provider::CanonicalTable;

use super::rollup::Sample;

/// Read `(created, content.<field>)` samples from `table` on `session`.
///
/// `field` names the numeric value inside the row's `content` document (e.g.
/// `"temp"`); rows lacking a numeric value at that field are skipped. Timestamps
/// come from each row's `created` instant, parsed to epoch microseconds. The
/// scoped session bounds which rows are read.
///
/// # Errors
/// Returns [`QueryError::Scan`] if the SurrealDB read fails.
pub async fn read_series(
    session: &Surreal<Db>,
    table: CanonicalTable,
    field: &str,
) -> Result<Vec<Sample>> {
    let surreal_table = table.surreal_table();
    let mut response = session
        .query(format!("SELECT * FROM {surreal_table}"))
        .await
        .map_err(|e| QueryError::Scan(e.to_string()))?;
    let rows: Vec<serde_json::Value> = response
        .take(0)
        .map_err(|e| QueryError::Scan(e.to_string()))?;

    Ok(rows.iter().filter_map(|row| sample_of(row, field)).collect())
}

/// Project one row into a [`Sample`], or `None` if it carries no usable value.
fn sample_of(row: &serde_json::Value, field: &str) -> Option<Sample> {
    let at_micros = row
        .get("created")
        .and_then(serde_json::Value::as_str)
        .and_then(crate::provider::parse_created_micros)?;
    let value = content_number(row, field)?;
    Some(Sample { at_micros, value })
}

/// Read a numeric `content.<field>` from a row, `None` when absent/non-numeric.
fn content_number(row: &serde_json::Value, field: &str) -> Option<f64> {
    row.get("content")?.get(field)?.as_f64()
}

#[cfg(test)]
mod tests {
    use super::sample_of;

    #[test]
    fn a_row_with_a_numeric_field_yields_a_sample() {
        let row = serde_json::json!({
            "created": "1970-01-01T00:01:00Z",
            "content": { "temp": 21.5 }
        });
        let sample = sample_of(&row, "temp").unwrap();
        assert_eq!(sample.at_micros, 60 * 1_000_000);
        assert!((sample.value - 21.5).abs() < f64::EPSILON);
    }

    #[test]
    fn a_row_missing_the_field_is_skipped() {
        let row = serde_json::json!({
            "created": "1970-01-01T00:00:00Z",
            "content": { "humidity": 40 }
        });
        assert_eq!(sample_of(&row, "temp"), None);
    }

    #[test]
    fn a_non_numeric_field_is_skipped() {
        let row = serde_json::json!({
            "created": "1970-01-01T00:00:00Z",
            "content": { "temp": "warm" }
        });
        assert_eq!(sample_of(&row, "temp"), None);
    }

    #[test]
    fn a_row_without_a_timestamp_is_skipped() {
        let row = serde_json::json!({ "content": { "temp": 1.0 } });
        assert_eq!(sample_of(&row, "temp"), None);
    }
}
