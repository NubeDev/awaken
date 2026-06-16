//! DataFusion query/compute surface for the rubix platform.
//!
//! The unification and compute layer (`rubix/docs/SCOPE.md`, "DataFusion — query
//! and compute"). DataFusion sits **above** SurrealDB only where cross-datasource
//! unification or heavy vectorized aggregation is wanted
//! (`rubix/STACK-DEISGN.md`, contract #6); SurrealQL stays first. This crate
//! provides three surfaces, all reading through a gate-issued scoped session so
//! SurrealDB row-level permissions decide which records are visible (contract
//! #1):
//!
//! - a **read-only SQL surface** over the canonical tables, guarded so only
//!   `SELECT`/`WITH` runs and gated by a WS-04 capability (contracts #1, #2);
//! - **vectorized time-window aggregation** (`avg/min/max/sum/count/first/last`
//!   over `minute…week` epoch-aligned buckets) that feeds rule decisions;
//! - a **vector / semantic-search surface** over SurrealDB vector columns.

mod aggregate;
mod error;
mod provider;
mod query;
mod search;
mod time;
mod transform;

pub use aggregate::{
    BucketRollup, Grain, Sample, SeriesFilter, rollup_window, rollup_window_filtered,
};
pub use error::{QueryError, Result};
pub use provider::{
    CanonicalTable, ContextCache, ScopeIdentity, build_context, build_context_cached,
};
pub use query::{ensure_read_only, run, run_authorized};
pub use search::{Neighbour, nearest};
pub use time::{ResolvedTimeScope, TimeBound, TimeScope, apply_time_scope, now_ms};
pub use transform::{Agg, CompareOp, ReduceCalc, Transform, apply_transforms};
