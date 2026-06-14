//! Typed rollup parameters. Aggregate and interval are closed enums that
//! render to fixed SQL fragments, so no caller input is interpolated into the
//! statement except point ids (bound as quoted literals after validation).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A time-bucketed aggregate over `his` for one or more points.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RollupSpec {
    /// Point ids (the `his.point_id` values) to roll up.
    pub points: Vec<String>,
    /// Bucket width.
    pub interval: Interval,
    /// Aggregate applied to the numeric value within each bucket.
    pub agg: Aggregate,
    /// Inclusive RFC 3339 lower bound on `ts`.
    pub start: Option<String>,
    /// Exclusive RFC 3339 upper bound on `ts`.
    pub end: Option<String>,
}

/// Supported bucket widths. Each maps to a DataFusion `INTERVAL` literal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Interval {
    Minute,
    FiveMinute,
    FifteenMinute,
    Hour,
    Day,
    Week,
}

impl Interval {
    /// The SQL `INTERVAL` literal for this bucket width.
    pub(crate) fn sql(self) -> &'static str {
        match self {
            Interval::Minute => "INTERVAL '1 minute'",
            Interval::FiveMinute => "INTERVAL '5 minutes'",
            Interval::FifteenMinute => "INTERVAL '15 minutes'",
            Interval::Hour => "INTERVAL '1 hour'",
            Interval::Day => "INTERVAL '1 day'",
            Interval::Week => "INTERVAL '7 days'",
        }
    }
}

/// Supported aggregate functions over the numeric value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Aggregate {
    Avg,
    Min,
    Max,
    Sum,
    Count,
    First,
    Last,
}

impl Aggregate {
    /// The SQL aggregate expression over the cast numeric `value`. `first`/
    /// `last` order by `ts` within the bucket; the rest are plain aggregates.
    pub(crate) fn sql(self) -> &'static str {
        match self {
            Aggregate::Avg => "avg(CAST(value AS DOUBLE))",
            Aggregate::Min => "min(CAST(value AS DOUBLE))",
            Aggregate::Max => "max(CAST(value AS DOUBLE))",
            Aggregate::Sum => "sum(CAST(value AS DOUBLE))",
            Aggregate::Count => "count(value)",
            Aggregate::First => "first_value(CAST(value AS DOUBLE) ORDER BY ts)",
            Aggregate::Last => "last_value(CAST(value AS DOUBLE) ORDER BY ts)",
        }
    }
}
