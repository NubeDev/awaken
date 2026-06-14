//! `rolling_mean/min/max/sum(time_col, col, window)` — time-duration windows.
//!
//! Windows are time-duration, not row-count (the design pins this: sensor data
//! is irregular, so a row-count window is rarely what an author means). The
//! implementation uses a DataFusion `RANGE` window frame over the time column,
//! looking back `window` seconds.
//!
//! Design-doc ambiguity resolved here: the doc's signature is `rolling_*(col,
//! window)` but a `RANGE` frame is undefined without naming the column it orders
//! and ranges over. Guessing a time column would be fragile, so the primitive
//! takes `time_col` explicitly as the first argument. The window result is
//! written into a new column `<col>_roll`; the row count is unchanged (a window
//! function emits one value per input row), so the frame never grows.

use super::compute::{quote_ident, require_column};
use super::duration::parse_seconds;
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Rolling mean of `col` over the trailing `window` (e.g. `"1h"`).
    pub fn rolling_mean(&self, time_col: &str, col: &str, window: &str) -> Result<Frame, RuleError> {
        self.rolling("avg", time_col, col, window)
    }

    /// Rolling minimum of `col` over the trailing `window`.
    pub fn rolling_min(&self, time_col: &str, col: &str, window: &str) -> Result<Frame, RuleError> {
        self.rolling("min", time_col, col, window)
    }

    /// Rolling maximum of `col` over the trailing `window`.
    pub fn rolling_max(&self, time_col: &str, col: &str, window: &str) -> Result<Frame, RuleError> {
        self.rolling("max", time_col, col, window)
    }

    /// Rolling sum of `col` over the trailing `window`.
    pub fn rolling_sum(&self, time_col: &str, col: &str, window: &str) -> Result<Frame, RuleError> {
        self.rolling("sum", time_col, col, window)
    }

    fn rolling(
        &self,
        agg: &str,
        time_col: &str,
        col: &str,
        window: &str,
    ) -> Result<Frame, RuleError> {
        require_column(self, time_col)?;
        require_column(self, col)?;
        let secs = parse_seconds(window)?;
        let out = format!("{col}_roll");
        let tq = quote_ident(time_col);
        let cq = quote_ident(col);
        // RANGE between `secs` ago and current row, ordered by the time column.
        // `extract(epoch from ..)` lets RANGE measure the bound in seconds
        // regardless of the time column's timestamp unit.
        let sql = format!(
            "SELECT *, {agg}({cq}) OVER ( \
                ORDER BY extract(epoch from {tq}) \
                RANGE BETWEEN {secs} PRECEDING AND CURRENT ROW \
             ) AS {out_q} FROM {TABLE}",
            out_q = quote_ident(&out)
        );
        self.compute(&sql)
    }
}
