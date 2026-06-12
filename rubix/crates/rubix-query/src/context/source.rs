//! The backing store a [`QueryEngine`](super::QueryEngine) reads canonical
//! tables from: SQLite on edge, Postgres under the `cloud` feature.
//!
//! Each variant builds the same read-only DataFusion `TableProvider`s for the
//! canonical tables, so the SQL surface above is identical regardless of where
//! the rows live. `his` over SQLite may union a Parquet cold tier; Postgres
//! reads `his` straight from the database.

use std::sync::Arc;

use datafusion::catalog::TableProvider;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::QueryError;
use crate::his::{HisTable, HisTier};
use crate::provider::SqliteTable;

#[cfg(feature = "cloud")]
use std::collections::HashMap;
#[cfg(feature = "cloud")]
use datafusion::sql::TableReference;
#[cfg(feature = "cloud")]
use datafusion_table_providers::postgres::PostgresTableFactory;
#[cfg(feature = "cloud")]
use datafusion_table_providers::sql::db_connection_pool::postgrespool::PostgresConnectionPool;

/// The store a query engine reads from.
#[derive(Clone)]
pub(super) enum Source {
    /// A read-tuned SQLite connection pool (edge default).
    Sqlite(Pool<SqliteConnectionManager>),
    /// A Postgres connection pool fronted by the DataFusion connector (cloud).
    #[cfg(feature = "cloud")]
    Postgres(Arc<PostgresConnectionPool>),
}

impl Source {
    /// The read-only `TableProvider` for one canonical `table`. For SQLite,
    /// `his` resolves through the two-tier union provider when `his_tier` is
    /// attached; Postgres always reads the live database table.
    pub(super) async fn canonical_provider(
        &self,
        table: &'static str,
        his_tier: &Option<HisTier>,
    ) -> Result<Arc<dyn TableProvider>, QueryError> {
        match self {
            Source::Sqlite(pool) => {
                if table == "his" {
                    if let Some(tier) = his_tier {
                        return Ok(Arc::new(HisTable::new(pool.clone(), tier.store())));
                    }
                }
                Ok(Arc::new(SqliteTable::try_new(pool.clone(), table)?))
            }
            #[cfg(feature = "cloud")]
            Source::Postgres(pool) => {
                let factory = PostgresTableFactory::new(pool.clone());
                factory
                    .table_provider(TableReference::bare(table))
                    .await
                    .map_err(|e| QueryError::Provider {
                        table,
                        message: e.to_string(),
                    })
            }
        }
    }
}

/// Open a Postgres-backed source over a `postgres://` connection string.
///
/// The connector's pool wants discrete `host`/`port`/`user`/`db`/`pass` params
/// (its `connection_string` param is libpq keyword form, not a URL), so the URL
/// is parsed with `tokio_postgres::Config` and mapped across. `sslmode` is
/// carried from the URL when present, else left to the connector default.
#[cfg(feature = "cloud")]
pub(super) async fn open_postgres(url: &str) -> Result<Source, QueryError> {
    use datafusion_table_providers::util::secrets::to_secret_map;
    use std::str::FromStr;
    use tokio_postgres::config::Host;

    let config = tokio_postgres::Config::from_str(url)
        .map_err(|e| QueryError::Pool(format!("invalid postgres url: {e}")))?;

    let mut params: HashMap<String, String> = HashMap::new();
    if let Some(Host::Tcp(host)) = config.get_hosts().first() {
        params.insert("host".to_string(), host.clone());
    }
    if let Some(port) = config.get_ports().first() {
        params.insert("port".to_string(), port.to_string());
    }
    if let Some(user) = config.get_user() {
        params.insert("user".to_string(), user.to_string());
    }
    if let Some(password) = config.get_password() {
        params.insert(
            "pass".to_string(),
            String::from_utf8_lossy(password).into_owned(),
        );
    }
    if let Some(dbname) = config.get_dbname() {
        params.insert("db".to_string(), dbname.to_string());
    }
    // Carry an explicit sslmode from the URL query string when given; the URL
    // parser does not surface it, so read it off the raw query.
    if let Some(sslmode) = sslmode_of(url) {
        params.insert("sslmode".to_string(), sslmode);
    }

    let pool = PostgresConnectionPool::new(to_secret_map(params))
        .await
        .map_err(|e| QueryError::Pool(e.to_string()))?;
    Ok(Source::Postgres(Arc::new(pool)))
}

/// The `sslmode` value from a `postgres://…?sslmode=…` query string, if present.
#[cfg(feature = "cloud")]
fn sslmode_of(url: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "sslmode").then(|| v.to_string())
    })
}
