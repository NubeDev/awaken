//! DataFusion SQL surface over the rubix store.
//!
//! One `SessionContext` registers the canonical tables (`sites`, `equips`,
//! `points`, `his`, `sparks`) from the SQLite database. The same context is
//! the single query surface for dashboards, reflow actors, and awaken tools.

mod context;
mod error;
mod sql;

pub use context::QueryEngine;
pub use error::QueryError;
pub use sql::QueryRows;
