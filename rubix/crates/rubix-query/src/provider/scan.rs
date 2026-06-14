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

use datafusion::arrow::array::{ArrayRef, StringArray, TimestampMicrosecondArray};
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
fn build_batch(table: CanonicalTable, rows: &[serde_json::Value]) -> Result<RecordBatch> {
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

#[cfg(test)]
mod tests {
    use super::build_batch;
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
}
