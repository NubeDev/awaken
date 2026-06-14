//! `describe()` — per-numeric-column summary statistics.
//!
//! Returns a small frame with one row per numeric column and the columns
//! `column`, `count`, `mean`, `min`, `max`, `stddev`. Output is bounded by the
//! number of input columns, so it never explodes — but it produces a fixed row
//! count unrelated to the input row count, so it goes through the reducing
//! compute path rather than the per-row no-growth guard.
//!
//! Implemented as a `UNION ALL` of one aggregate row per column.

use super::compute::quote_ident;
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Summarize each numeric column: count, mean, min, max, stddev.
    pub fn describe(&self) -> Result<Frame, RuleError> {
        let numeric: Vec<String> = self
            .schema()
            .fields()
            .iter()
            .filter(|f| f.data_type().is_numeric())
            .map(|f| f.name().clone())
            .collect();
        if numeric.is_empty() {
            return Err(RuleError::Runtime(
                "describe: frame has no numeric columns".into(),
            ));
        }
        let parts = numeric
            .iter()
            .map(|c| {
                let cq = quote_ident(c);
                let lit = c.replace('\'', "''");
                format!(
                    "SELECT '{lit}' AS column, count({cq}) AS count, \
                     avg({cq}) AS mean, min({cq}) AS min, max({cq}) AS max, \
                     stddev({cq}) AS stddev FROM {TABLE}"
                )
            })
            .collect::<Vec<_>>()
            .join(" UNION ALL ");
        self.compute_reduce(&parts)
    }
}
