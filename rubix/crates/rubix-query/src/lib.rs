//! DataFusion SQL surface over the rubix store.
//!
//! A `SessionContext` registers the canonical tables (`sites`, `equips`,
//! `points`, `his`, `sparks`) from the SQLite database via a custom read-only
//! `TableProvider` that reads schema from `PRAGMA table_info` (so empty tables
//! still resolve their columns) and rows live per scan. The same surface
//! serves dashboards, reflow actors, and awaken tools.

mod context;
mod error;
mod provider;
mod sql;

pub use context::QueryEngine;
pub use error::QueryError;
pub use sql::QueryRows;
