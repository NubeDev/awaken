//! The query engine: a SQLite connection pool that builds a fresh DataFusion
//! `SessionContext` per query, so registered tables always reflect the live
//! schema and committed data (an empty table still resolves its columns).

mod open;
mod register;
mod scope;
mod scoped;
mod tables;

pub use scope::QueryScope;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::his::HisTier;

/// A DataFusion SQL surface over the rubix store.
///
/// Cheap to clone (the underlying connection pool is reference-counted). Built
/// once per process from the same SQLite database the HTTP store writes to.
/// When a [`HisTier`] is attached, `his` resolves as the union of the SQLite
/// recent tier and the Parquet cold tier; otherwise `his` is SQLite-only.
#[derive(Clone)]
pub struct QueryEngine {
    pool: Pool<SqliteConnectionManager>,
    his_tier: Option<HisTier>,
}

impl QueryEngine {
    /// Attach a Parquet cold tier, so `his` queries union the SQLite recent
    /// tier with the Parquet partitions. Without it `his` stays SQLite-only.
    pub fn with_his_tier(mut self, tier: HisTier) -> Self {
        self.his_tier = Some(tier);
        self
    }
}
