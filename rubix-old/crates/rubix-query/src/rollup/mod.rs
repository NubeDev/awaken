//! Time-bucketed aggregates over `his` — the analytics primitive dashboards
//! and rule boards use for trends.

mod run;
mod spec;

pub use spec::{Aggregate, Interval, RollupSpec};
