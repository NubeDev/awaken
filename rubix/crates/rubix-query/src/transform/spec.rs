//! The transform spec — the portable, hybrid-execution contract (§1).
//!
//! Transforms are a declarative post-query pipeline stored on the chart record
//! (`rubix/docs/design/DASHBOARDS-SCOPE.md` §1). The spec is the durable
//! portability lever; *where* each op runs is a per-op choice that can change:
//!
//! - **Aggregate ops** (`filter`/`groupBy`/`reduce`) change row cardinality and
//!   shrink the wire, so they run **server-side** here (a small DataFusion stage
//!   over the result batches — they need the full dataset).
//! - **Cosmetic ops** (`rename`/`calculated`/`organize`) are row-by-row or
//!   column-reorder and run **client-side** via nexus's executor — keeping builder
//!   edits instant. The backend treats them as no-ops so the full spec can ride
//!   the request unchanged (the contract stays whole; only execution splits).
//!
//! The variants mirror nexus's `Transform` union verbatim so the client executor
//! and the stored spec stay one shape.

/// A binary comparison operator for [`Transform::Filter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
}

impl CompareOp {
    /// Parse a wire operator token (`=`, `!=`, `>`, `>=`, `<`, `<=`).
    #[must_use]
    pub fn parse(raw: &str) -> Option<CompareOp> {
        match raw {
            "=" => Some(CompareOp::Eq),
            "!=" => Some(CompareOp::Ne),
            ">" => Some(CompareOp::Gt),
            ">=" => Some(CompareOp::Ge),
            "<" => Some(CompareOp::Lt),
            "<=" => Some(CompareOp::Le),
            _ => None,
        }
    }

    /// The SQL spelling of this operator.
    #[must_use]
    pub fn sql(self) -> &'static str {
        match self {
            CompareOp::Eq => "=",
            CompareOp::Ne => "<>",
            CompareOp::Gt => ">",
            CompareOp::Ge => ">=",
            CompareOp::Lt => "<",
            CompareOp::Le => "<=",
        }
    }
}

/// An aggregation function for [`Transform::GroupBy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agg {
    Sum,
    Avg,
    Min,
    Max,
    Count,
}

impl Agg {
    /// Parse a wire aggregation token.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Agg> {
        match raw {
            "sum" => Some(Agg::Sum),
            "avg" => Some(Agg::Avg),
            "min" => Some(Agg::Min),
            "max" => Some(Agg::Max),
            "count" => Some(Agg::Count),
            _ => None,
        }
    }

    /// The SQL aggregate function name.
    #[must_use]
    pub fn sql_func(self) -> &'static str {
        match self {
            Agg::Sum => "sum",
            Agg::Avg => "avg",
            Agg::Min => "min",
            Agg::Max => "max",
            Agg::Count => "count",
        }
    }
}

/// A reduce calculation for [`Transform::Reduce`] — collapses every row to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceCalc {
    First,
    Last,
    Sum,
    Avg,
    Min,
    Max,
    Count,
}

impl ReduceCalc {
    /// Parse a wire reduce token.
    #[must_use]
    pub fn parse(raw: &str) -> Option<ReduceCalc> {
        match raw {
            "first" => Some(ReduceCalc::First),
            "last" => Some(ReduceCalc::Last),
            "sum" => Some(ReduceCalc::Sum),
            "avg" => Some(ReduceCalc::Avg),
            "min" => Some(ReduceCalc::Min),
            "max" => Some(ReduceCalc::Max),
            "count" => Some(ReduceCalc::Count),
            _ => None,
        }
    }
}

/// One transform in the pipeline. Mirrors nexus's `Transform` union; the backend
/// executes only the aggregate variants (see [`Transform::is_aggregate`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transform {
    /// Cosmetic: copy column `from` into `to`, drop `from`. Client-side.
    Rename { from: String, to: String },
    /// Cosmetic: add `field = left <op> right`. Client-side.
    Calculated {
        field: String,
        left: String,
        op: String,
        right: String,
    },
    /// Aggregate: keep rows where `field <op> value`. Server-side.
    Filter {
        field: String,
        op: CompareOp,
        value: String,
    },
    /// Aggregate: one row per distinct `by`, aggregating `field` into `as_`.
    GroupBy {
        by: String,
        field: String,
        agg: Agg,
        as_: String,
    },
    /// Aggregate: collapse all rows to one holding `calc(field)` as `as_`.
    Reduce {
        field: String,
        calc: ReduceCalc,
        as_: String,
    },
    /// Cosmetic: reorder columns to follow `order`. Client-side.
    Organize { order: Vec<String> },
}

impl Transform {
    /// Whether this op changes row cardinality and so runs server-side (§1).
    ///
    /// `filter`/`groupBy`/`reduce` are aggregate; the rest are cosmetic and run
    /// client-side via nexus's executor.
    #[must_use]
    pub fn is_aggregate(&self) -> bool {
        matches!(
            self,
            Transform::Filter { .. } | Transform::GroupBy { .. } | Transform::Reduce { .. }
        )
    }
}
