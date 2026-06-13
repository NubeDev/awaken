//! `resample(time_col, every, aggs)` — downsample onto a fixed time grid.
//!
//! Implemented as `date_bin` + group-by (what a Timescale user expects), per the
//! design: bucket the time column into `every`-wide bins and aggregate each
//! requested column within the bin. `aggs` maps a source column to an aggregate
//! (`avg`/`min`/`max`/`sum`/`count`); the output has one row per bin, so the
//! frame can only shrink — never explode.

use super::compute::{quote_ident, require_column};
use super::duration::parse_seconds;
use super::{Frame, TABLE};
use crate::error::RuleError;

/// The aggregates a `resample` column may request. Closed set, so an unknown
/// function is a clean runtime error rather than arbitrary injected SQL.
fn render_agg(func: &str, col: &str) -> Result<String, RuleError> {
    let cq = quote_ident(col);
    let expr = match func {
        "avg" | "mean" => format!("avg({cq})"),
        "min" => format!("min({cq})"),
        "max" => format!("max({cq})"),
        "sum" => format!("sum({cq})"),
        "count" => format!("count({cq})"),
        other => {
            return Err(RuleError::Runtime(format!(
                "resample: unknown aggregate `{other}` for `{col}` \
                 (use avg, min, max, sum, count)"
            )))
        }
    };
    Ok(format!("{expr} AS {cq}"))
}

impl Frame {
    /// Bin `time_col` into `every`-wide buckets and aggregate `aggs` per bucket.
    ///
    /// `aggs` is `(column, aggregate)` pairs. The binned timestamp is returned in
    /// `time_col`, and there is one row per non-empty bin.
    pub fn resample(
        &self,
        time_col: &str,
        every: &str,
        aggs: &[(String, String)],
    ) -> Result<Frame, RuleError> {
        require_column(self, time_col)?;
        if aggs.is_empty() {
            return Err(RuleError::Runtime(
                "resample: no aggregates given".into(),
            ));
        }
        let secs = parse_seconds(every)?;
        let tq = quote_ident(time_col);
        let bin = format!("date_bin(INTERVAL '{secs} seconds', {tq})");

        let mut projections = vec![format!("{bin} AS {tq}")];
        for (col, func) in aggs {
            require_column(self, col)?;
            projections.push(render_agg(func, col)?);
        }
        let select = projections.join(", ");
        let sql = format!(
            "SELECT {select} FROM {TABLE} GROUP BY {bin} ORDER BY {bin}"
        );
        self.compute(&sql)
    }
}
