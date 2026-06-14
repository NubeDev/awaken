//! Vector / semantic search over SurrealDB vector columns.
//!
//! Vectors live beside the records (`rubix/docs/SCOPE.md`, principle 6), so
//! nearest-neighbour search runs in SurrealDB on the principal's scoped session —
//! SurrealQL first (`rubix/STACK-DEISGN.md`, contract #6), not DataFusion.

mod nearest;

pub use nearest::{Neighbour, nearest};
