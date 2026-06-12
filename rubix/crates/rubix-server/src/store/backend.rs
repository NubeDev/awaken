//! Store backend selection. STACK-DEISGN.md "Postgres (cloud), SQLite (edge)":
//! one store API, two backends. The edge build compiles SQLite only; the
//! `cloud` feature adds a synchronous Postgres backend. The rest of the server
//! calls `Store` methods synchronously regardless of backend.

use r2d2_sqlite::SqliteConnectionManager;

/// A pooled relational backend. SQLite is always present; Postgres is compiled
/// in only under the `cloud` feature and is selected at runtime by a
/// `postgres://` connection URL.
#[derive(Clone)]
pub(crate) enum Backend {
    Sqlite(r2d2::Pool<SqliteConnectionManager>),
    #[cfg(feature = "cloud")]
    Postgres(r2d2::Pool<r2d2_postgres::PostgresConnectionManager<postgres::NoTls>>),
}

/// A checked-out SQLite connection.
pub(crate) type SqliteConn = r2d2::PooledConnection<SqliteConnectionManager>;

/// A checked-out Postgres connection.
#[cfg(feature = "cloud")]
pub(crate) type PostgresConn =
    r2d2::PooledConnection<r2d2_postgres::PostgresConnectionManager<postgres::NoTls>>;
