use r2d2_sqlite::SqliteConnectionManager;

use super::backend::Backend;
use super::schema::SCHEMA_SQLITE;
use super::Result;

/// Pooled relational store. Cheap to clone; one pool per process. Wraps either
/// the SQLite (edge) or Postgres (cloud) [`Backend`]; the rest of the server
/// calls the same synchronous methods on it regardless.
#[derive(Clone)]
pub struct Store {
    pub(crate) backend: Backend,
}

impl Store {
    /// Open the SQLite backend at `path`. The edge default.
    pub fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;",
            )
        });
        let pool = r2d2::Pool::builder().build(manager)?;
        let mut conn = pool.get()?;
        // The base schema establishes a fresh database (idempotent CREATEs);
        // the migration ladder evolves an existing one (column adds, backfills)
        // so a schema change never requires deleting the file. See `migrate`.
        conn.execute_batch(SCHEMA_SQLITE)?;
        super::migrate::run(&mut conn)?;
        drop(conn);
        Ok(Self {
            backend: Backend::Sqlite(pool),
        })
    }

    /// Open a store from a connection string, dispatching on scheme. A
    /// `postgres://` (or `postgresql://`) URL selects the Postgres backend
    /// (cloud feature only); anything else is treated as a SQLite file path.
    /// STACK-DEISGN.md "Postgres (cloud), SQLite (edge)".
    pub fn connect(target: &str) -> anyhow::Result<Self> {
        if is_postgres_url(target) {
            return Self::connect_postgres(target);
        }
        Self::open(std::path::Path::new(target))
    }

    /// True when `target` is a Postgres connection URL.
    pub fn is_postgres_target(target: &str) -> bool {
        is_postgres_url(target)
    }

    #[cfg(feature = "cloud")]
    fn connect_postgres(url: &str) -> anyhow::Result<Self> {
        use super::schema::SCHEMA_POSTGRES;
        let config: postgres::Config = url
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid postgres url: {e}"))?;
        let manager = r2d2_postgres::PostgresConnectionManager::new(config, postgres::NoTls);
        let pool = r2d2::Pool::builder().build(manager)?;
        pool.get()?.batch_execute(SCHEMA_POSTGRES)?;
        Ok(Self {
            backend: Backend::Postgres(pool),
        })
    }

    #[cfg(not(feature = "cloud"))]
    fn connect_postgres(_url: &str) -> anyhow::Result<Self> {
        anyhow::bail!(
            "a postgres:// store url requires the cloud profile; rebuild with --features cloud"
        )
    }

    /// Check out a SQLite connection. Panics-free: errors map to [`StoreError`].
    /// Only valid on the SQLite backend; the Postgres paths use
    /// [`postgres_conn`](Self::postgres_conn).
    pub(crate) fn sqlite_conn(&self) -> Result<super::backend::SqliteConn> {
        match &self.backend {
            Backend::Sqlite(pool) => Ok(pool.get()?),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => Err(super::StoreError::Db(anyhow::anyhow!(
                "sqlite connection requested on a postgres backend"
            ))),
        }
    }

    /// Wipe every table on the Postgres backend. Test-only support for the
    /// shared store suite, which runs each Postgres pass against a clean slate.
    #[cfg(feature = "cloud")]
    pub fn truncate_all_for_tests(&self) -> Result<()> {
        super::postgres::truncate_all(self)
    }

    /// Check out a Postgres connection.
    #[cfg(feature = "cloud")]
    pub(crate) fn postgres_conn(&self) -> Result<super::backend::PostgresConn> {
        match &self.backend {
            Backend::Postgres(pool) => Ok(pool.get()?),
            Backend::Sqlite(_) => Err(super::StoreError::Db(anyhow::anyhow!(
                "postgres connection requested on a sqlite backend"
            ))),
        }
    }
}

/// True when a connection string names the Postgres backend.
fn is_postgres_url(target: &str) -> bool {
    target.starts_with("postgres://") || target.starts_with("postgresql://")
}
