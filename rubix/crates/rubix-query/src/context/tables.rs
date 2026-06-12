//! The canonical tables exposed on the SQL surface.
//!
//! These mirror the rubix store schema one-to-one and are the stable names
//! dashboards, reflow actors, and awaken tools query against.

/// SQLite table names registered into the DataFusion default catalog under
/// the same bare name, so `SELECT * FROM points` resolves directly.
pub(super) const CANONICAL: &[&str] = &["sites", "equips", "points", "his", "sparks"];
