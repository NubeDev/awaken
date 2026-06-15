//! Render Arrow query results into JSON rows for the wire.
//!
//! `rubix-query` returns DataFusion [`RecordBatch`]es — the engine's native row
//! shape (`rubix/docs/sessions/WS-16.md`: "the transport layer renders them to
//! the wire"). This verb serialises each batch to JSON objects keyed by column
//! name using the arrow JSON writer, the one place batch→wire conversion lives.

use arrow_json::writer::{JsonArray, WriterBuilder};
use datafusion::arrow::record_batch::RecordBatch;
use serde_json::Value;

/// Convert query result `batches` into one JSON object per row.
///
/// Returns an empty vector for an empty result. Each row is a JSON object whose
/// keys are the result column names.
///
/// # Errors
/// Returns the arrow JSON serialisation error if a batch cannot be written, or
/// the parse error if the writer's output is not valid JSON (it always is, but
/// the boundary is checked rather than unwrapped).
pub fn batches_to_rows(batches: &[RecordBatch]) -> Result<Vec<Value>, String> {
    let mut buffer = Vec::new();
    {
        let mut writer = WriterBuilder::new()
            .with_explicit_nulls(true)
            .build::<_, JsonArray>(&mut buffer);
        for batch in batches {
            writer.write(batch).map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
    }
    if buffer.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&buffer).map_err(|e| e.to_string())
}
