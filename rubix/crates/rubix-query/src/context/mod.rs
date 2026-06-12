//! The query engine: a SQLite connection pool that builds a fresh DataFusion
//! `SessionContext` per query, so registered tables always reflect the live
//! schema and committed data (an empty table still resolves its columns).

mod open;
mod register;
mod tables;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

/// A DataFusion SQL surface over the rubix store.
///
/// Cheap to clone (the underlying connection pool is reference-counted). Built
/// once per process from the same SQLite database the HTTP store writes to.
#[derive(Clone)]
pub struct QueryEngine {
    pool: Pool<SqliteConnectionManager>,
}
