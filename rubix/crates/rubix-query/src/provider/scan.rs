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
    let mut response = session
        .query(format!("SELECT * FROM {surreal_table}"))
        .await
        .map_err(|e| QueryError::Scan(e.to_string()))?;
    let rows: Vec<serde_json::Value> = response
        .take(0)
        .map_err(|e| QueryError::Scan(e.to_string()))?;

    build_batch(table, &rows)
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
/// SurrealDB serialises a `datetime` to an RFC3339 string in JSON; this parses
/// that into the epoch-microsecond representation the Arrow timestamp column
/// uses, which the window-bucket math then aligns (see `crate::aggregate`).
fn micros_field(row: &serde_json::Value, field: &str) -> Option<i64> {
    let raw = match row.get(field) {
        Some(serde_json::Value::String(s)) => s,
        _ => return None,
    };
    parse_rfc3339_micros(raw)
}

/// Parse an RFC3339 timestamp into microseconds since the Unix epoch.
///
/// Implemented without a date library: SurrealDB always emits UTC RFC3339 with a
/// `Z` (or `+00:00`) offset for stored datetimes, so a fixed-shape parser is
/// sufficient and avoids a new dependency for one field. Returns `None` for any
/// string that is not that shape rather than guessing.
fn parse_rfc3339_micros(raw: &str) -> Option<i64> {
    let bytes = raw.as_bytes();
    // YYYY-MM-DDTHH:MM:SS is the minimum prefix.
    if bytes.len() < 19 || bytes[4] != b'-' || bytes[10] != b'T' {
        return None;
    }
    let year: i64 = raw.get(0..4)?.parse().ok()?;
    let month: u32 = raw.get(5..7)?.parse().ok()?;
    let day: u32 = raw.get(8..10)?.parse().ok()?;
    let hour: i64 = raw.get(11..13)?.parse().ok()?;
    let minute: i64 = raw.get(14..16)?.parse().ok()?;
    let second: i64 = raw.get(17..19)?.parse().ok()?;

    let days = days_from_civil(year, month, day)?;
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second;
    let micros = fractional_micros(&raw[19..]);
    Some(secs * 1_000_000 + micros)
}

/// Days since the Unix epoch for a civil date, by Howard Hinnant's algorithm.
fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(month);
    let d = i64::from(day);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Microseconds from the fractional-seconds tail of an RFC3339 string.
///
/// Accepts `.ffffff` of up to six digits before the timezone marker; any tail
/// that is not a fractional part contributes zero (the seconds resolution is
/// still correct).
fn fractional_micros(tail: &str) -> i64 {
    let Some(rest) = tail.strip_prefix('.') else {
        return 0;
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).take(6).collect();
    if digits.is_empty() {
        return 0;
    }
    let scale = 10_i64.pow(6 - digits.len() as u32);
    digits.parse::<i64>().unwrap_or(0) * scale
}

#[cfg(test)]
mod tests {
    use super::{build_batch, parse_rfc3339_micros};
    use crate::provider::schema::CanonicalTable;

    #[test]
    fn epoch_start_parses_to_zero() {
        assert_eq!(parse_rfc3339_micros("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn a_known_instant_parses_to_its_epoch_micros() {
        // 2021-01-01T00:00:00Z == 1609459200 seconds since the epoch.
        assert_eq!(
            parse_rfc3339_micros("2021-01-01T00:00:00Z"),
            Some(1_609_459_200_000_000)
        );
    }

    #[test]
    fn fractional_seconds_contribute_micros() {
        assert_eq!(
            parse_rfc3339_micros("1970-01-01T00:00:00.5Z"),
            Some(500_000)
        );
        assert_eq!(
            parse_rfc3339_micros("1970-01-01T00:00:00.000123Z"),
            Some(123)
        );
    }

    #[test]
    fn a_malformed_timestamp_is_none() {
        assert_eq!(parse_rfc3339_micros("not-a-date"), None);
        assert_eq!(parse_rfc3339_micros("2021/01/01"), None);
    }

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
