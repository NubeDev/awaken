//! The `his` two-tier history surface: a SQLite recent (hot) tier unioned with
//! a Parquet cold tier on an object store.
//!
//! STACK-DEISGN.md specifies a Parquet history store via `object_store` (edge +
//! cloud tiers) and a `his` `TableProvider` over Parquet partitions. The SQLite
//! `his` table stays the hot/recent tier; aged rows flush into dated per-point
//! Parquet partitions, and [`HisTable`] unions both so queries are unchanged
//! across the boundary.

mod hot;
mod partition;
mod read;
mod schema;
mod table;
mod tier;
mod write;

pub(crate) use table::HisTable;
pub use tier::HisTier;
pub use write::{write_partitions, HisRow};
