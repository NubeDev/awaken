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
    /// `connection_string` is either a `postgres://`/`postgresql://` URL (the form
    /// the compose file and Makefile advertise) or a libpq `key=value` string. A
    /// URL is decomposed into the discrete pool parameters, honoring an `?sslmode=`
    /// query (the pool defaults to `verify-full` when absent, so a non-TLS local
    /// database needs `?sslmode=disable`). The pool is established now, so a
    /// register that reaches this far has a live backend; a query later just
    /// materialises a provider per table.
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
        let config = DatasourceConfig::new(id, label).with_kind("postgres");
        let params = pool_params(connection_string);
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

/// Build the connection-pool parameter map from `connection_string`.
///
/// `datafusion-table-providers` only parses libpq `key=value` strings out of its
/// `connection_string` parameter, so a `postgres://` URL handed to it whole is
/// rejected as an invalid configuration. This decomposes a URL into the discrete
/// `host`/`port`/`user`/`pass`/`db` parameters the pool understands, carrying
/// through any recognised SSL query parameters; a non-URL string is passed
/// through unchanged as a libpq `connection_string`.
fn pool_params(connection_string: &str) -> HashMap<String, String> {
    let Ok(url) = url::Url::parse(connection_string) else {
        return HashMap::from([("connection_string".to_owned(), connection_string.to_owned())]);
    };
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        return HashMap::from([("connection_string".to_owned(), connection_string.to_owned())]);
    }

    let mut params = HashMap::new();
    if let Some(host) = url.host_str() {
        params.insert("host".to_owned(), host.to_owned());
    }
    if let Some(port) = url.port() {
        params.insert("port".to_owned(), port.to_string());
    }
    // Userinfo is used verbatim; credentials containing reserved characters
    // should be supplied via the libpq `key=value` form instead.
    if !url.username().is_empty() {
        params.insert("user".to_owned(), url.username().to_owned());
    }
    if let Some(pass) = url.password() {
        params.insert("pass".to_owned(), pass.to_owned());
    }
    let db = url.path().trim_start_matches('/');
    if !db.is_empty() {
        params.insert("db".to_owned(), db.to_owned());
    }
    // Carry through the SSL / application-name query parameters the pool reads.
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "sslmode" | "sslrootcert" | "application_name" => {
                params.insert(key.into_owned(), value.into_owned());
            }
            _ => {}
        }
    }
    params
}
