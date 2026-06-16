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

/// An equality filter on a row's `content`: keep only rows whose
/// `content.<key>` equals `value` (as a string).
///
/// The series read selects a numeric `content.<field>`, but many rows can share
/// that field across different categories — every reading stores its number at
/// `content.value` regardless of `content.measure`, for instance. A filter
/// narrows the series to one category (`measure == "temp"`) so a rule decides on
/// just that metric rather than a blend.
#[derive(Debug, Clone, Copy)]
pub struct SeriesFilter<'a> {
    /// The `content` key the filter matches on (e.g. `"measure"`).
    pub key: &'a str,
    /// The exact string value `content.<key>` must equal.
    pub value: &'a str,
}

/// Read `(created, content.<field>)` samples from `table` on `session`, keeping
/// only rows that pass `filter` (when set).
///
/// `field` names the numeric value inside the row's `content` document (e.g.
/// `"temp"`); rows lacking a numeric value at that field are skipped. `filter`,
/// when present, additionally requires `content.<filter.key>` to equal
/// `filter.value` — so a single numeric field shared across categories
/// (`content.value` across measures) can be narrowed to one. Timestamps come from
/// each row's `created` instant, parsed to epoch microseconds. The scoped session
/// bounds which rows are read.
///
/// # Errors
/// Returns [`QueryError::Scan`] if the SurrealDB read fails.
pub async fn read_series(
    session: &Surreal<Db>,
    table: CanonicalTable,
    field: &str,
    filter: Option<SeriesFilter<'_>>,
) -> Result<Vec<Sample>> {
    let surreal_table = table.surreal_table();
    let mut response = session
        .query(format!("SELECT * FROM {surreal_table}"))
        .await
        .map_err(|e| QueryError::Scan(e.to_string()))?;
    let rows: Vec<serde_json::Value> = response
        .take(0)
        .map_err(|e| QueryError::Scan(e.to_string()))?;

    Ok(rows
        .iter()
        .filter(|row| passes(row, filter))
        .filter_map(|row| sample_of(row, field))
        .collect())
}

/// Whether `row` satisfies `filter` — always true when no filter is set, else
/// `content.<key>` must equal `value` as a string.
fn passes(row: &serde_json::Value, filter: Option<SeriesFilter<'_>>) -> bool {
    match filter {
        None => true,
        Some(SeriesFilter { key, value }) => row
            .get("content")
            .and_then(|c| c.get(key))
            .and_then(serde_json::Value::as_str)
            == Some(value),
    }
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
    use super::{SeriesFilter, passes, sample_of};

    #[test]
    fn no_filter_passes_every_row() {
        let row = serde_json::json!({ "content": { "measure": "co2" } });
        assert!(passes(&row, None));
    }

    #[test]
    fn a_matching_filter_passes_and_a_mismatch_is_excluded() {
        let row = serde_json::json!({ "content": { "measure": "temp", "value": 21.5 } });
        let keep = SeriesFilter { key: "measure", value: "temp" };
        let drop = SeriesFilter { key: "measure", value: "co2" };
        assert!(passes(&row, Some(keep)));
        assert!(!passes(&row, Some(drop)));
    }

    #[test]
    fn a_missing_filter_key_is_excluded() {
        let row = serde_json::json!({ "content": { "value": 21.5 } });
        let filter = SeriesFilter { key: "measure", value: "temp" };
        assert!(!passes(&row, Some(filter)));
    }

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
