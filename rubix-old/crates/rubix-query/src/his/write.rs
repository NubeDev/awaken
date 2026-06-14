//! Write aged `his` samples into Parquet partitions on an object store.
//!
//! Samples are grouped by (point, UTC day); each group becomes one Parquet file
//! under its partition prefix. This is the cold-tier write the server's flush
//! path calls once rows age out of the SQLite hot tier.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use datafusion::arrow::array::StringBuilder;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::object_store::{ObjectStore, ObjectStoreExt, PutPayload};
use datafusion::parquet::arrow::ArrowWriter;

use super::partition::{day_of, partition_path};
use super::schema::his_schema;
use crate::error::QueryError;

/// One history row to persist: the point it belongs to, its timestamp, and the
/// JSON-encoded value text (identical encoding to the SQLite `his.value`).
#[derive(Debug, Clone)]
pub struct HisRow {
    pub point_id: String,
    pub ts: DateTime<Utc>,
    pub value: String,
}

/// Write `rows` into Parquet partitions on `store`, one file per (point, day).
///
/// Returns the number of partition files written. `flush` is the instant used
/// for the leaf file name, so distinct flushes of the same day never collide.
/// Empty input writes nothing and returns zero.
pub async fn write_partitions(
    store: &Arc<dyn ObjectStore>,
    rows: &[HisRow],
    flush: DateTime<Utc>,
) -> Result<usize, QueryError> {
    let mut groups: BTreeMap<(String, NaiveDate), Vec<&HisRow>> = BTreeMap::new();
    for row in rows {
        groups
            .entry((row.point_id.clone(), day_of(&row.ts)))
            .or_default()
            .push(row);
    }

    let mut written = 0;
    for ((point_id, day), group) in groups {
        let payload = encode(&group)?;
        let path = partition_path(&point_id, day, &flush);
        store
            .put(&path, PutPayload::from(payload))
            .await
            .map_err(|e| QueryError::His(format!("put partition {path}: {e}")))?;
        written += 1;
    }
    Ok(written)
}

/// Encode one partition group as a Parquet byte buffer over the `his` schema.
fn encode(group: &[&HisRow]) -> Result<Vec<u8>, QueryError> {
    let schema = his_schema();
    let mut point = StringBuilder::new();
    let mut ts = StringBuilder::new();
    let mut value = StringBuilder::new();
    for row in group {
        point.append_value(&row.point_id);
        ts.append_value(ts_text(&row.ts));
        value.append_value(&row.value);
    }
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(point.finish()),
            Arc::new(ts.finish()),
            Arc::new(value.finish()),
        ],
    )
    .map_err(QueryError::Encode)?;

    let mut buf = Vec::new();
    let mut writer = ArrowWriter::try_new(&mut buf, schema, None)
        .map_err(|e| QueryError::His(format!("open parquet writer: {e}")))?;
    writer
        .write(&batch)
        .map_err(|e| QueryError::His(format!("write parquet batch: {e}")))?;
    writer
        .close()
        .map_err(|e| QueryError::His(format!("close parquet writer: {e}")))?;
    Ok(buf)
}

/// Render a timestamp the same way the SQLite store does (RFC 3339, microsecond
/// precision, `Z` suffix) so hot- and cold-tier `ts` strings sort and compare
/// identically.
fn ts_text(ts: &DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}
