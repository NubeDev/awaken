//! Build a [`QueryEngine`] by registering the canonical tables from SQLite.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use datafusion::prelude::SessionContext;
use datafusion::sql::TableReference;
use datafusion_table_providers::sql::db_connection_pool::sqlitepool::SqliteConnectionPoolFactory;
use datafusion_table_providers::sql::db_connection_pool::Mode;
use datafusion_table_providers::sqlite::SqliteTableFactory;

use super::tables::CANONICAL;
use super::QueryEngine;
use crate::error::QueryError;

/// How long a query may wait for a pooled SQLite connection.
const POOL_TIMEOUT: Duration = Duration::from_millis(5000);

impl QueryEngine {
    /// Open the query engine over the SQLite database at `path`.
    ///
    /// Each canonical table is registered as a DataFusion `TableProvider` under
    /// its bare name. The database is opened read-only from the engine's point
    /// of view — writes still flow through the HTTP store and priority array.
    pub async fn open(path: &Path) -> Result<Self, QueryError> {
        let db = path.to_string_lossy();
        let pool = SqliteConnectionPoolFactory::new(&db, Mode::File, POOL_TIMEOUT)
            .build()
            .await
            .map_err(|e| QueryError::Pool(e.to_string()))?;
        let factory = SqliteTableFactory::new(Arc::new(pool));

        let ctx = SessionContext::new();
        for &table in CANONICAL {
            let provider = factory
                .table_provider(TableReference::bare(table))
                .await
                .map_err(|e| QueryError::Provider {
                    table,
                    message: e.to_string(),
                })?;
            ctx.register_table(table, provider)
                .map_err(|source| QueryError::Register { table, source })?;
        }

        Ok(Self { ctx: Arc::new(ctx) })
    }
}
