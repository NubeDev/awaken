//! Read the SQLite `his` hot tier into an Arrow batch over the canonical
//! schema. This is the recent tier the cold Parquet partitions union with.

use std::sync::Arc;

use datafusion::arrow::array::StringBuilder;
use datafusion::arrow::record_batch::RecordBatch;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use super::schema::his_schema;
use crate::error::QueryError;

/// Load every SQLite `his` row into one batch (`point_id`, `ts`, `value`).
///
/// All three columns are read as text, matching the cold-tier encoding so the
/// union is schema-identical regardless of which tier a row came from.
pub(crate) fn read_hot_batch(
    pool: &Pool<SqliteConnectionManager>,
) -> Result<RecordBatch, QueryError> {
    let conn = pool.get().map_err(|e| QueryError::Pool(e.to_string()))?;
    let mut stmt = conn
        .prepare("SELECT point_id, ts, value FROM his")
        .map_err(backend)?;
    let mut rows = stmt.query([]).map_err(backend)?;

    let mut point = StringBuilder::new();
    let mut ts = StringBuilder::new();
    let mut value = StringBuilder::new();
    while let Some(row) = rows.next().map_err(backend)? {
        point.append_value(row.get::<_, String>(0).map_err(backend)?);
        ts.append_value(row.get::<_, String>(1).map_err(backend)?);
        value.append_value(row.get::<_, String>(2).map_err(backend)?);
    }

    RecordBatch::try_new(
        his_schema(),
        vec![
            Arc::new(point.finish()),
            Arc::new(ts.finish()),
            Arc::new(value.finish()),
        ],
    )
    .map_err(QueryError::Encode)
}

fn backend(e: rusqlite::Error) -> QueryError {
    QueryError::Backend(e.to_string())
}
