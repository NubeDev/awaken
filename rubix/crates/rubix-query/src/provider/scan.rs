//! Read one canonical table through the scoped session into an Arrow batch.
//!
//! Contract #6 (`rubix/STACK-DEISGN.md`): SurrealDB does the row read; DataFusion
//! sits above. So the scan runs a plain `SELECT * FROM <table>` on the
//! principal's scoped SurrealDB session — SurrealDB row-level permissions decide
//! which rows come back, never an app filter (contract #1) — then projects each
//! row into the table's fixed Arrow schema (see [`super::schema`]). Decoding goes
//! through SurrealDB's JSON form so a schemaless document maps uniformly: the
//! structural columns are read by name and the whole row is preserved as the
//! `content` JSON string, so a query can still reach any free-form field.

use std::sync::Arc;

use datafusion::arrow::array::{
    ArrayRef, Float64Array, StringArray, TimestampMicrosecondArray,
};
use datafusion::arrow::record_batch::RecordBatch;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{QueryError, Result};

use super::instant::parse_created_micros;
use super::schema::CanonicalTable;

/// Scan `table` on `session` into a single Arrow [`RecordBatch`].
///
/// `session` is a gate-issued scoped connection; the rows returned are exactly
/// the ones its SurrealDB permissions admit. An empty or not-yet-written table
/// yields a zero-row batch (the schema is still well-formed), never an error, so
/// a query over a table no writer has populated returns no rows.
///
/// # Errors
/// Returns [`QueryError::Scan`] if the SurrealDB read fails, or
/// [`QueryError::DataFusion`] if the decoded columns cannot form a batch.
pub async fn scan_table(session: &Surreal<Db>, table: CanonicalTable) -> Result<RecordBatch> {
    let surreal_table = table.surreal_table();
    let outcome = session
        .query(format!("SELECT * FROM {surreal_table}"))
        .await
        .and_then(|mut response| response.take::<Vec<serde_json::Value>>(0));
    let rows = match outcome {
        Ok(rows) => rows,
        // A canonical table no workstream has declared yet (e.g. `insight`
        // before its WS-11 writer) is legitimately empty, not an error: a
        // read-only scan over an absent table yields zero rows.
        Err(error) if is_missing_table(&error) => Vec::new(),
        Err(error) => return Err(QueryError::Scan(error.to_string())),
    };

    build_batch(table, &rows)
}

/// Whether a SurrealDB error reports that the scanned table does not exist.
///
/// Matched on the engine's message because SurrealDB does not surface a typed
/// "missing table" variant here; the read is otherwise infallible for an empty
/// table, so the narrow string match cannot mask a different failure.
fn is_missing_table(error: &surrealdb::Error) -> bool {
    let message = error.to_string();
    message.contains("does not exist") && message.contains("table")
}

/// Project decoded JSON rows into the table's Arrow schema.
///
/// [`CanonicalTable::Readings`] has its own typed time-series shape, so it is
/// projected into the 6-column reading schema (`id/namespace/series/at/value/
/// created`); every other table uses the shared structural-plus-`content`
/// projection (`rubix/docs/design/READINGS-TIMESERIES.md`, "Read path").
fn build_batch(table: CanonicalTable, rows: &[serde_json::Value]) -> Result<RecordBatch> {
    if table == CanonicalTable::Readings {
        return build_reading_batch(table, rows);
    }
    let mut ids: Vec<String> = Vec::with_capacity(rows.len());
    let mut namespaces: Vec<Option<String>> = Vec::with_capacity(rows.len());
    let mut created: Vec<Option<i64>> = Vec::with_capacity(rows.len());
    let mut updated: Vec<Option<i64>> = Vec::with_capacity(rows.len());
    let mut content: Vec<Option<String>> = Vec::with_capacity(rows.len());

    for row in rows {
        ids.push(row_id(row));
        namespaces.push(string_field(row, "namespace"));
        created.push(micros_field(row, "created"));
        updated.push(micros_field(row, "updated"));
        content.push(Some(row.to_string()));
    }

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(ids)),
        Arc::new(StringArray::from(namespaces)),
        Arc::new(TimestampMicrosecondArray::from(created)),
        Arc::new(TimestampMicrosecondArray::from(updated)),
        Arc::new(StringArray::from(content)),
    ];

    RecordBatch::try_new(table.arrow_schema(), columns).map_err(|e| QueryError::DataFusion(e.into()))
}

/// Project decoded JSON rows into the 6-column reading schema.
///
/// A reading's hot columns are typed, so this surfaces `series`, the
/// measurement instant `at`, and the numeric `value` as their own Arrow columns
/// rather than burying them in a `content` blob. `at`/`created` are read with
/// the shared [`micros_field`] datetime helper; `value` as an f64; `series` as
/// the bare register id with its `record:` prefix and `⟨⟩` brackets stripped so
/// it matches a register's id (`rubix/docs/design/READINGS-TIMESERIES.md`,
/// "Read path").
fn build_reading_batch(table: CanonicalTable, rows: &[serde_json::Value]) -> Result<RecordBatch> {
    let mut ids: Vec<String> = Vec::with_capacity(rows.len());
    let mut namespaces: Vec<Option<String>> = Vec::with_capacity(rows.len());
    let mut series: Vec<Option<String>> = Vec::with_capacity(rows.len());
    let mut at: Vec<Option<i64>> = Vec::with_capacity(rows.len());
    let mut value: Vec<Option<f64>> = Vec::with_capacity(rows.len());
    let mut created: Vec<Option<i64>> = Vec::with_capacity(rows.len());

    for row in rows {
        ids.push(row_id(row));
        namespaces.push(string_field(row, "namespace"));
        series.push(series_field(row, "series"));
        at.push(micros_field(row, "at"));
        value.push(f64_field(row, "value"));
        created.push(micros_field(row, "created"));
    }

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(ids)),
        Arc::new(StringArray::from(namespaces)),
        Arc::new(StringArray::from(series)),
        Arc::new(TimestampMicrosecondArray::from(at)),
        Arc::new(Float64Array::from(value)),
        Arc::new(TimestampMicrosecondArray::from(created)),
    ];

    RecordBatch::try_new(table.arrow_schema(), columns).map_err(|e| QueryError::DataFusion(e.into()))
}

/// The row's `id` as a string. SurrealDB renders a record id as `table:key` in
/// JSON; an absent id falls back to the empty string so the non-null column
/// invariant holds (a row always has an id at the store boundary).
fn row_id(row: &serde_json::Value) -> String {
    match row.get("id") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Read a string field by name, `None` when absent or not a string.
fn string_field(row: &serde_json::Value, field: &str) -> Option<String> {
    match row.get(field) {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Read an RFC3339 datetime field as microseconds since the Unix epoch.
///
/// Delegates to [`parse_created_micros`] so the scan and the rollup series reader
/// share one datetime parser (`docs/FILE-LAYOUT.md`, dedup).
fn micros_field(row: &serde_json::Value, field: &str) -> Option<i64> {
    match row.get(field) {
        Some(serde_json::Value::String(s)) => parse_created_micros(s),
        _ => None,
    }
}

/// Read a numeric field by name as an f64, `None` when absent or non-numeric.
fn f64_field(row: &serde_json::Value, field: &str) -> Option<f64> {
    row.get(field).and_then(serde_json::Value::as_f64)
}

/// Read a `series` record link as its bare id string.
///
/// SurrealDB renders a record link as `record:abc` or, when the key is a UUID or
/// otherwise non-identifier, `record:⟨…⟩` (the U+27E8/U+27E9 angle brackets). The
/// reading's `series` must match a register's bare id, so this strips a leading
/// `record:` table prefix and any surrounding `⟨ ⟩` brackets. A non-string value
/// (e.g. an object link) is `None`.
fn series_field(row: &serde_json::Value, field: &str) -> Option<String> {
    let raw = row.get(field).and_then(serde_json::Value::as_str)?;
    let bare = raw.split_once(':').map_or(raw, |(_, key)| key);
    let bare = bare
        .strip_prefix('\u{27e8}')
        .and_then(|s| s.strip_suffix('\u{27e9}'))
        .unwrap_or(bare);
    Some(bare.to_string())
}

#[cfg(test)]
mod tests {
    use super::{build_batch, series_field};
    use crate::provider::schema::CanonicalTable;

    #[test]
    fn an_empty_table_builds_a_zero_row_batch() {
        let batch = build_batch(CanonicalTable::Records, &[]).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 5);
    }

    #[test]
    fn a_row_projects_its_structural_columns() {
        let rows = vec![serde_json::json!({
            "id": "record:abc",
            "namespace": "tenant-a",
            "created": "1970-01-01T00:00:00Z",
            "updated": "1970-01-01T00:00:01Z",
            "content": { "temp": 21.5 }
        })];
        let batch = build_batch(CanonicalTable::Records, &rows).unwrap();
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn an_empty_readings_table_builds_a_zero_row_six_col_batch() {
        let batch = build_batch(CanonicalTable::Readings, &[]).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 6);
    }

    #[test]
    fn a_reading_row_projects_its_series_at_and_value_columns() {
        use datafusion::arrow::array::{
            Float64Array, StringArray, TimestampMicrosecondArray,
        };

        let rows = vec![serde_json::json!({
            "id": "reading:abc",
            "namespace": "tenant-a",
            "series": "record:reg1",
            "at": "1970-01-01T00:01:00Z",
            "value": 21.5,
            "created": "1970-01-01T00:00:00Z"
        })];
        let batch = build_batch(CanonicalTable::Readings, &rows).unwrap();
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 6);

        let series = batch
            .column(2)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(series.value(0), "reg1");

        let at = batch
            .column(3)
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .unwrap();
        assert_eq!(at.value(0), 60 * 1_000_000);

        let value = batch
            .column(4)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert!((value.value(0) - 21.5).abs() < f64::EPSILON);
    }

    #[test]
    fn series_field_strips_the_record_prefix_and_angle_brackets() {
        let plain = serde_json::json!({ "series": "record:reg1" });
        assert_eq!(series_field(&plain, "series"), Some("reg1".to_string()));

        let bracketed = serde_json::json!({ "series": "record:\u{27e8}abc-123\u{27e9}" });
        assert_eq!(series_field(&bracketed, "series"), Some("abc-123".to_string()));

        let missing = serde_json::json!({});
        assert_eq!(series_field(&missing, "series"), None);
    }
}
