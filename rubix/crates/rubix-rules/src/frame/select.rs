//! `select(cols)` — project a subset of columns.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Keep only `cols`, in the given order. A projection never adds rows.
    pub fn select(&self, cols: &[String]) -> Result<Frame, RuleError> {
        if cols.is_empty() {
            return Err(RuleError::Runtime("select: no columns given".into()));
        }
        for c in cols {
            require_column(self, c)?;
        }
        let list = cols
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");
        self.compute(&format!("SELECT {list} FROM {TABLE}"))
    }
}
