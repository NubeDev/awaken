//! Vectorized time-window aggregation that feeds rule decisions.
//!
//! A rule decides on time-window rollups of a numeric series; DataFusion owns
//! that data, Rhai owns the decision (`rubix/STACK-DEISGN.md`). This module reads
//! a numeric series from a canonical table through the principal's scoped session
//! and rolls it up into epoch-aligned buckets — `avg/min/max/sum/count/first/
//! last` over `minute…week` grains. The bucketing math ([`window`]) and the
//! aggregation fold ([`rollup`]) are pure and unit-tested; [`series`] is the
//! scoped scan that produces the samples.

mod rollup;
mod series;
mod window;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::Result;
use crate::provider::CanonicalTable;

pub use rollup::{BucketRollup, Sample};
pub use series::SeriesFilter;
pub use window::Grain;

use rollup::rollup;
use series::read_series;

/// Roll a numeric `content.<field>` series from `table` up into per-bucket
/// aggregates at `grain`, scoped to the principal of `session`.
///
/// The series is read through the scoped session (SurrealDB permissions decide
/// the rows, contract #1), then folded into epoch-aligned buckets in ascending
/// order. Rows whose `content.<field>` is absent or non-numeric are excluded, so
/// the rollup reflects only real readings. An empty series yields no buckets.
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if the scoped scan fails.
pub async fn rollup_window(
    session: &Surreal<Db>,
    table: CanonicalTable,
    field: &str,
    grain: Grain,
) -> Result<Vec<BucketRollup>> {
    rollup_window_filtered(session, table, field, grain, None).await
}

/// Like [`rollup_window`], but narrows the series to rows passing `filter`.
///
/// A single numeric field can be shared across categories — every reading stores
/// its number at `content.value` regardless of `content.measure`. A
/// [`SeriesFilter`] (`measure == "temp"`) restricts the rollup to one category so
/// a rule decides on just that metric rather than a blend of all of them. A
/// `None` filter is exactly [`rollup_window`].
///
/// # Errors
/// Returns a [`QueryError`](crate::QueryError) if the scoped scan fails.
pub async fn rollup_window_filtered(
    session: &Surreal<Db>,
    table: CanonicalTable,
    field: &str,
    grain: Grain,
    filter: Option<SeriesFilter<'_>>,
) -> Result<Vec<BucketRollup>> {
    let samples = read_series(session, table, field, filter).await?;
    Ok(rollup(&samples, grain))
}
