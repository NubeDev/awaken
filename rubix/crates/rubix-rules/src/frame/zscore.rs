//! `zscore(col)` — standard score of a column against the frame's mean/stddev.
//!
//! Writes a new column `<col>_z = (col - mean) / stddev` computed with windowed
//! aggregates over the whole frame (an unbounded window), so it stays a single
//! pass that emits one value per row. Row count is unchanged.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Append `<col>_z`, the z-score of `col` over the full frame.
    pub fn zscore(&self, col: &str) -> Result<Frame, RuleError> {
        require_column(self, col)?;
        let cq = quote_ident(col);
        let out = quote_ident(&format!("{col}_z"));
        // `stddev` is the sample stddev; guard a zero/null spread by emitting 0
        // rather than a NULL/inf so downstream thresholds stay well-defined.
        let sql = format!(
            "SELECT *, CASE \
                WHEN stddev({cq}) OVER () IS NULL \
                  OR stddev({cq}) OVER () = 0 THEN 0.0 \
                ELSE ({cq} - avg({cq}) OVER ()) / (stddev({cq}) OVER ()) \
             END AS {out} FROM {TABLE}"
        );
        self.compute(&sql)
    }
}
