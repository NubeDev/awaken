//! `any_true(col)` — reduce a boolean column to a single bool.
//!
//! The bridge from a vectorized flag column (e.g. `<col>_anomaly` from
//! `anomalies`) to the script's decision logic. The reduction runs in the engine
//! (a `bool_or` aggregate), so the script never iterates rows to find a flag.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Whether any row's boolean column `col` is true (false on empty/all-null).
    pub fn any_true(&self, col: &str) -> Result<bool, RuleError> {
        require_column(self, col)?;
        let sql = format!(
            "SELECT coalesce(bool_or({}), false) AS hit FROM {TABLE}",
            quote_ident(col)
        );
        let out = self.compute_reduce(&sql)?;
        read_single_bool(&out)
    }
}

/// Read the single boolean cell from a one-row, one-column result frame.
fn read_single_bool(frame: &Frame) -> Result<bool, RuleError> {
    use datafusion::arrow::array::BooleanArray;
    let batch = frame
        .batches()
        .iter()
        .find(|b| b.num_rows() > 0)
        .ok_or_else(|| RuleError::Engine("any_true: empty result".into()))?;
    let col = batch.column(0);
    let arr = col
        .as_any()
        .downcast_ref::<BooleanArray>()
        .ok_or_else(|| RuleError::Engine("any_true: result not boolean".into()))?;
    Ok(arr.value(0))
}
