//! `head(n)` / `tail(n)` — keep the first / last `n` rows.
//!
//! Both shrink the frame (or leave it unchanged when `n` exceeds the row
//! count); neither can grow it. `tail` orders by row position descending via a
//! `row_number` window, then re-orders ascending so the original order is kept.

use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Keep the first `n` rows.
    pub fn head(&self, n: i64) -> Result<Frame, RuleError> {
        let n = check_n(n)?;
        self.compute(&format!("SELECT * FROM {TABLE} LIMIT {n}"))
    }

    /// Keep the last `n` rows, preserving their original order.
    pub fn tail(&self, n: i64) -> Result<Frame, RuleError> {
        let n = check_n(n)?;
        let total = self.row_count();
        let skip = total.saturating_sub(n as usize);
        self.compute(&format!("SELECT * FROM {TABLE} LIMIT {n} OFFSET {skip}"))
    }
}

fn check_n(n: i64) -> Result<i64, RuleError> {
    if n < 0 {
        Err(RuleError::Runtime(format!("row count must be >= 0, got {n}")))
    } else {
        Ok(n)
    }
}
