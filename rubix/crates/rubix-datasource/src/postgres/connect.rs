//! Build a Postgres `TableProvider` via `datafusion-table-providers`.
//!
//! Connects a pooled Postgres client from a libpq connection string and exposes
//! the declared tables as DataFusion providers (`rubix/docs/SCOPE.md`,
//! "Datasources"). The connection pool is built once when the connector is
//! constructed; each declared table is materialised into a provider through the
//! crate's `PostgresTableFactory`. This whole module is `#[cfg(feature =
//! "postgres")]` — absent the feature it never compiles in, so the connector fails
//! closed rather than degrading at runtime (`rubix/STACK-DEISGN.md`).

use std::collections::HashMap;
use std::sync::Arc;

use datafusion::datasource::TableProvider;
use datafusion::sql::TableReference;
use datafusion_table_providers::postgres::PostgresTableFactory;
use datafusion_table_providers::sql::db_connection_pool::postgrespool::PostgresConnectionPool;
use datafusion_table_providers::util::secrets::to_secret_map;

use crate::connector::{Connector, DatasourceConfig};
use crate::error::{DatasourceError, Result};

/// A connector over a Postgres database, exposing a declared set of tables.
pub struct PostgresConnector {
    config: DatasourceConfig,
    factory: PostgresTableFactory,
    tables: Vec<String>,
}

impl PostgresConnector {
    /// Connect to Postgres and declare the `tables` this datasource exposes.
    ///
    /// `connection_string` is a libpq-style string (`host=… user=… dbname=…`, or
    /// a `postgres://` URL). The pool is established now, so a register that
    /// reaches this far has a live backend; a query later just materialises a
    /// provider per table.
    ///
    /// # Errors
    /// Returns [`DatasourceError::Connect`] if the connection pool cannot be built
    /// (unreachable host, bad credentials, …).
    pub async fn connect(
        id: impl Into<String>,
        label: impl Into<String>,
        connection_string: &str,
        tables: Vec<String>,
    ) -> Result<Self> {
        let config = DatasourceConfig::new(id, label);
        let params = HashMap::from([(
            "connection_string".to_owned(),
            connection_string.to_owned(),
        )]);
        let pool = PostgresConnectionPool::new(to_secret_map(params))
            .await
            .map_err(|e| DatasourceError::Connect {
                id: config.id().to_owned(),
                reason: e.to_string(),
            })?;
        Ok(Self {
            config,
            factory: PostgresTableFactory::new(Arc::new(pool)),
            tables,
        })
    }
}

impl Connector for PostgresConnector {
    fn config(&self) -> &DatasourceConfig {
        &self.config
    }

    fn tables(&self) -> Vec<String> {
        self.tables.clone()
    }

    async fn table_provider(&self, table: &str) -> Result<Arc<dyn TableProvider>> {
        self.factory
            .table_provider(TableReference::bare(table.to_owned()))
            .await
            .map_err(|e| DatasourceError::Connect {
                id: self.config.id().to_owned(),
                reason: e.to_string(),
            })
    }
}
