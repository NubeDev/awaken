//! The query engine: a connection source that builds a fresh DataFusion
//! `SessionContext` per query, so registered tables always reflect the live
//! schema and committed data (an empty table still resolves its columns).
//!
//! The source is SQLite on edge and Postgres under the `cloud` feature. Either
//! way the canonical tables (`sites`, `equips`, `points`, `his`, `sparks`) are
//! registered as read-only providers and the same SQL surface — unscoped or
//! tenant-scoped — runs over them.

mod open;
mod register;
mod scope;
mod scoped;
mod source;
mod tables;

pub use scope::QueryScope;

use crate::his::HisTier;
use source::Source;

/// A DataFusion SQL surface over the rubix store.
///
/// Cheap to clone (the underlying connection pool is reference-counted). Built
/// once per process from the same database the HTTP store writes to. When a
/// [`HisTier`] is attached (SQLite only), `his` resolves as the union of the
/// SQLite recent tier and the Parquet cold tier; otherwise `his` is read whole
/// from the backing store.
#[derive(Clone)]
pub struct QueryEngine {
    source: Source,
    his_tier: Option<HisTier>,
}

impl QueryEngine {
    /// Attach a Parquet cold tier, so `his` queries union the SQLite recent
    /// tier with the Parquet partitions. Without it `his` stays single-tier.
    /// Only meaningful for the SQLite source.
    pub fn with_his_tier(mut self, tier: HisTier) -> Self {
        self.his_tier = Some(tier);
        self
    }
}
