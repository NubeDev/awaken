//! DataFusion SQL surface over the rubix store.
//!
//! A `SessionContext` registers the canonical tables (`sites`, `equips`,
//! `points`, `his`, `sparks`) from the SQLite database via a custom read-only
//! `TableProvider` that reads schema from `PRAGMA table_info` (so empty tables
//! still resolve their columns) and rows live per scan. The same surface
//! serves dashboards, reflow actors, and awaken tools.

mod context;
mod error;
mod his;
mod provider;
mod rollup;
mod sql;

pub use context::{QueryEngine, QueryScope};
pub use error::QueryError;
pub use his::{write_partitions, HisRow, HisTier};
pub use rollup::{Aggregate, Interval, RollupSpec};
pub use sql::QueryRows;
