//! Build a [`QueryEngine`] over a SQLite database file.

use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use super::QueryEngine;
use crate::error::QueryError;

impl QueryEngine {
    /// Open the query engine over the SQLite database at `path`.
    ///
    /// Builds a read-tuned connection pool (WAL, query-only); each
    /// [`query`](Self::query) builds a fresh context with the canonical tables,
    /// so schema and data are always read live. Writes still flow through the
    /// HTTP store and priority array; this surface is read-only.
    pub async fn open(path: &Path) -> Result<Self, QueryError> {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA query_only = ON; \
                 PRAGMA busy_timeout = 5000;",
            )
        });
        let pool = Pool::builder()
            .build(manager)
            .map_err(|e| QueryError::Pool(e.to_string()))?;
        Ok(Self { pool })
    }
}
