//! Read the `his` Parquet cold tier back into Arrow batches.
//!
//! Lists every partition file under the root prefix and decodes each into
//! batches typed to the canonical `his` schema. The union table provider
//! concatenates these with the SQLite hot tier so a query spans both.

use std::sync::Arc;

use bytes::Bytes;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::object_store::{ObjectStore, ObjectStoreExt};
use datafusion::parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use futures::StreamExt;

use super::partition::root_prefix;
use crate::error::QueryError;

/// Read all Parquet partitions on `store` into batches over the `his` schema.
///
/// Returns an empty vector when no partitions exist yet (a fresh tier). The
/// caller projects/filters the union downstream, so this returns every row.
pub(crate) async fn read_partitions(
    store: &Arc<dyn ObjectStore>,
) -> Result<Vec<RecordBatch>, QueryError> {
    let prefix = root_prefix();
    let mut listing = store.list(Some(&prefix));

    let mut batches = Vec::new();
    while let Some(entry) = listing.next().await {
        let meta = entry.map_err(|e| QueryError::His(format!("list partitions: {e}")))?;
        if !meta.location.as_ref().ends_with(".parquet") {
            continue;
        }
        let bytes = store
            .get(&meta.location)
            .await
            .map_err(|e| QueryError::His(format!("get {}: {e}", meta.location)))?
            .bytes()
            .await
            .map_err(|e| QueryError::His(format!("read {}: {e}", meta.location)))?;
        decode_into(&bytes, &mut batches)?;
    }
    Ok(batches)
}

/// Decode one Parquet file's batches, appending them to `out`.
fn decode_into(bytes: &Bytes, out: &mut Vec<RecordBatch>) -> Result<(), QueryError> {
    let builder = ParquetRecordBatchReaderBuilder::try_new(bytes.clone())
        .map_err(|e| QueryError::His(format!("open parquet reader: {e}")))?;
    let reader = builder
        .build()
        .map_err(|e| QueryError::His(format!("build parquet reader: {e}")))?;
    for batch in reader {
        out.push(batch.map_err(QueryError::Encode)?);
    }
    Ok(())
}
