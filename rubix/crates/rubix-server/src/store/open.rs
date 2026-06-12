use r2d2_sqlite::SqliteConnectionManager;

use super::schema::SCHEMA;
use super::Result;

/// Pooled SQLite store. Cheap to clone; one pool per process.
#[derive(Clone)]
pub struct Store {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Store {
    pub fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;",
            )
        });
        let pool = r2d2::Pool::builder().build(manager)?;
        pool.get()?.execute_batch(SCHEMA)?;
        Ok(Self { pool })
    }

    pub(crate) fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }
}
