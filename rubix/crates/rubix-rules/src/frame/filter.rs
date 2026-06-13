//! `filter_gt` / `filter_lt` / `filter_eq(col, value)` — keep matching rows.
//!
//! A filter only ever removes rows, so it cannot explode the frame. The
//! comparison value is numeric and rendered as a SQL float literal (no string
//! interpolation of untrusted text into the predicate); the column is quoted.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Keep rows where `col` > `value`.
    pub fn filter_gt(&self, col: &str, value: f64) -> Result<Frame, RuleError> {
        self.filter(col, ">", value)
    }

    /// Keep rows where `col` < `value`.
    pub fn filter_lt(&self, col: &str, value: f64) -> Result<Frame, RuleError> {
        self.filter(col, "<", value)
    }

    /// Keep rows where `col` = `value`.
    pub fn filter_eq(&self, col: &str, value: f64) -> Result<Frame, RuleError> {
        self.filter(col, "=", value)
    }

    fn filter(&self, col: &str, op: &str, value: f64) -> Result<Frame, RuleError> {
        require_column(self, col)?;
        if !value.is_finite() {
            return Err(RuleError::Runtime(format!(
                "filter: comparison value must be finite, got {value}"
            )));
        }
        let lit = render_float(value);
        self.compute(&format!(
            "SELECT * FROM {TABLE} WHERE {} {op} {lit}",
            quote_ident(col)
        ))
    }
}

/// Render a finite float as a SQL literal that DataFusion parses as a number.
fn render_float(value: f64) -> String {
    // `{:?}` keeps a decimal point (e.g. `25.0`) so the literal is typed double,
    // and round-trips finite values without scientific-notation surprises.
    format!("{value:?}")
}
