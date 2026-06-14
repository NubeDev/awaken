//! `lag` / `diff` / `pct_change(time_col, col)` — row-to-row deltas.
//!
//! Each uses a `lag(col) OVER (ORDER BY time_col)` window — one value per input
//! row, so the row count is unchanged. As with `rolling_*`, the ordering column
//! is taken explicitly (a window's ordering is undefined otherwise; this is the
//! same doc-signature resolution noted in `rolling.rs`).
//!
//! - `lag`        → `<col>_lag`: the previous value.
//! - `diff`       → `<col>_diff`: `col - previous`.
//! - `pct_change` → `<col>_pct`: `(col - previous) / previous`.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Append `<col>_lag`, the prior row's value of `col` by `time_col` order.
    pub fn lag(&self, time_col: &str, col: &str) -> Result<Frame, RuleError> {
        self.windowed(time_col, col, "lag")
    }

    /// Append `<col>_diff`, `col` minus its prior value.
    pub fn diff(&self, time_col: &str, col: &str) -> Result<Frame, RuleError> {
        self.windowed(time_col, col, "diff")
    }

    /// Append `<col>_pct`, the fractional change from the prior value.
    pub fn pct_change(&self, time_col: &str, col: &str) -> Result<Frame, RuleError> {
        self.windowed(time_col, col, "pct")
    }

    fn windowed(&self, time_col: &str, col: &str, suffix: &str) -> Result<Frame, RuleError> {
        require_column(self, time_col)?;
        require_column(self, col)?;
        let tq = quote_ident(time_col);
        let cq = quote_ident(col);
        let prev = format!("lag({cq}) OVER (ORDER BY {tq})");
        let out = quote_ident(&format!("{col}_{suffix}"));
        let value = match suffix {
            "lag" => prev.clone(),
            "diff" => format!("{cq} - {prev}"),
            "pct" => format!(
                "CASE WHEN {prev} IS NULL OR {prev} = 0 THEN NULL \
                 ELSE ({cq} - {prev}) / {prev} END"
            ),
            _ => unreachable!("windowed suffix is internal"),
        };
        self.compute(&format!("SELECT *, {value} AS {out} FROM {TABLE}"))
    }
}
