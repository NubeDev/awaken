//! Read a table's Arrow schema from `PRAGMA table_info` — correct even when
//! the table is empty, unlike data-driven inference.

use std::sync::Arc;

use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use rusqlite::Connection;

use crate::error::QueryError;

/// Map a SQLite declared type to an Arrow type. SQLite is dynamically typed;
/// we pick the affinity that round-trips losslessly through `arrow-json`.
/// Unknown/empty decltypes fall back to `Utf8`.
fn arrow_type(decl: &str) -> DataType {
    let d = decl.to_ascii_uppercase();
    if d.contains("INT") {
        DataType::Int64
    } else if d.contains("REAL") || d.contains("FLOA") || d.contains("DOUB") {
        DataType::Float64
    } else if d.contains("BOOL") {
        DataType::Boolean
    } else {
        // TEXT, BLOB, NUMERIC, dateless, or empty: keep as text.
        DataType::Utf8
    }
}

/// Build the Arrow schema for `table` from its column metadata.
pub(super) fn table_schema(conn: &Connection, table: &str) -> Result<SchemaRef, QueryError> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql).map_err(sqlite_err)?;
    let rows = stmt
        .query_map([], |row| {
            // table_info columns: cid, name, type, notnull, dflt_value, pk.
            // Every field is Arrow-nullable: projections, filters, and future
            // outer joins legitimately yield nulls regardless of the SQLite
            // NOT NULL constraint, which governs writes, not query results.
            let name: String = row.get(1)?;
            let decl: String = row.get(2)?;
            Ok(Field::new(name, arrow_type(&decl), true))
        })
        .map_err(sqlite_err)?;
    let fields = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(sqlite_err)?;
    if fields.is_empty() {
        return Err(QueryError::Backend(format!("no such table: {table}")));
    }
    Ok(Arc::new(Schema::new(fields)))
}

fn sqlite_err(e: rusqlite::Error) -> QueryError {
    QueryError::Backend(e.to_string())
}
