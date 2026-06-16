//! Render Arrow query results into JSON rows for the wire.
//!
//! `rubix-query` returns DataFusion [`RecordBatch`]es — the engine's native row
//! shape (`rubix/docs/sessions/WS-16.md`: "the transport layer renders them to
//! the wire"). This verb serialises each batch to JSON objects keyed by column
//! name using the arrow JSON writer, the one place batch→wire conversion lives.

use arrow_json::writer::{JsonArray, WriterBuilder};
use datafusion::arrow::datatypes::DataType;
use datafusion::arrow::record_batch::RecordBatch;
use serde_json::Value;

use crate::dto::query::ColumnDto;

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

/// Derive the result columns (name + coarse type) from the result schema.
///
/// The client gets column types without sniffing rows (§3/§7,
/// `rubix/docs/design/DASHBOARDS-SCOPE.md`), which also lets a board render an
/// empty result with the right axes. The schema is taken from the first batch;
/// every batch of one result shares it. An empty result (no batches) has no
/// schema to report, so the column list is empty.
#[must_use]
pub fn columns_of(batches: &[RecordBatch]) -> Vec<ColumnDto> {
    let Some(first) = batches.first() else {
        return Vec::new();
    };
    first
        .schema()
        .fields()
        .iter()
        .map(|field| ColumnDto {
            name: field.name().clone(),
            kind: type_tag(field.data_type()).to_owned(),
        })
        .collect()
}

/// A coarse, client-friendly type tag for an Arrow [`DataType`].
fn type_tag(data_type: &DataType) -> &'static str {
    use DataType::{
        Boolean, Date32, Date64, Float16, Float32, Float64, Int8, Int16, Int32, Int64, LargeUtf8,
        Timestamp, UInt8, UInt16, UInt32, UInt64, Utf8,
    };
    match data_type {
        Int8 | Int16 | Int32 | Int64 | UInt8 | UInt16 | UInt32 | UInt64 | Float16 | Float32
        | Float64 => "number",
        Utf8 | LargeUtf8 => "string",
        Boolean => "boolean",
        Timestamp(_, _) | Date32 | Date64 => "timestamp",
        _ => "other",
    }
}
