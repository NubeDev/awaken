//! Read all rows of a table into a single Arrow `RecordBatch`, typed to the
//! table's schema.

use std::sync::Arc;

use datafusion::arrow::array::{
    ArrayRef, BooleanBuilder, Float64Builder, Int64Builder, StringBuilder,
};
use datafusion::arrow::datatypes::{DataType, SchemaRef};
use datafusion::arrow::record_batch::RecordBatch;
use rusqlite::types::ValueRef;
use rusqlite::Connection;

use crate::error::QueryError;

/// Per-column Arrow builder, dispatched on the schema's declared type.
enum Col {
    Int(Int64Builder),
    Float(Float64Builder),
    Bool(BooleanBuilder),
    Text(StringBuilder),
}

impl Col {
    fn for_type(ty: &DataType) -> Self {
        match ty {
            DataType::Int64 => Col::Int(Int64Builder::new()),
            DataType::Float64 => Col::Float(Float64Builder::new()),
            DataType::Boolean => Col::Bool(BooleanBuilder::new()),
            _ => Col::Text(StringBuilder::new()),
        }
    }

    /// Append a SQLite cell, coercing to the column's Arrow type. A cell whose
    /// runtime type cannot coerce is stored as null rather than failing the
    /// whole query (SQLite is dynamically typed).
    fn push(&mut self, v: ValueRef<'_>) {
        match self {
            Col::Int(b) => match v {
                ValueRef::Integer(i) => b.append_value(i),
                ValueRef::Real(r) => b.append_value(r as i64),
                _ => b.append_null(),
            },
            Col::Float(b) => match v {
                ValueRef::Real(r) => b.append_value(r),
                ValueRef::Integer(i) => b.append_value(i as f64),
                _ => b.append_null(),
            },
            Col::Bool(b) => match v {
                ValueRef::Integer(i) => b.append_value(i != 0),
                _ => b.append_null(),
            },
            Col::Text(b) => match v {
                ValueRef::Null => b.append_null(),
                ValueRef::Text(t) => b.append_value(String::from_utf8_lossy(t)),
                ValueRef::Integer(i) => b.append_value(i.to_string()),
                ValueRef::Real(r) => b.append_value(r.to_string()),
                ValueRef::Blob(_) => b.append_null(),
            },
        }
    }

    fn finish(self) -> ArrayRef {
        match self {
            Col::Int(mut b) => Arc::new(b.finish()),
            Col::Float(mut b) => Arc::new(b.finish()),
            Col::Bool(mut b) => Arc::new(b.finish()),
            Col::Text(mut b) => Arc::new(b.finish()),
        }
    }
}

/// Load every row of `table` into one batch shaped by `schema`.
pub(super) fn read_batch(
    conn: &Connection,
    table: &str,
    schema: SchemaRef,
) -> Result<RecordBatch, QueryError> {
    let mut cols: Vec<Col> = schema
        .fields()
        .iter()
        .map(|f| Col::for_type(f.data_type()))
        .collect();

    let sql = format!("SELECT * FROM {table}");
    let mut stmt = conn.prepare(&sql).map_err(backend)?;
    let mut rows = stmt.query([]).map_err(backend)?;
    while let Some(row) = rows.next().map_err(backend)? {
        for (i, col) in cols.iter_mut().enumerate() {
            col.push(row.get_ref(i).map_err(backend)?);
        }
    }

    let arrays: Vec<ArrayRef> = cols.into_iter().map(Col::finish).collect();
    RecordBatch::try_new(schema, arrays).map_err(QueryError::Encode)
}

fn backend(e: rusqlite::Error) -> QueryError {
    QueryError::Backend(e.to_string())
}
