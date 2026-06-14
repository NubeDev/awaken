//! `sort(col, ascending)` — order rows by a column.
//!
//! A pure reordering: row count is unchanged.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Sort by `col`, ascending when `ascending` is true.
    pub fn sort(&self, col: &str, ascending: bool) -> Result<Frame, RuleError> {
        require_column(self, col)?;
        let dir = if ascending { "ASC" } else { "DESC" };
        self.compute(&format!(
            "SELECT * FROM {TABLE} ORDER BY {} {dir}",
            quote_ident(col)
        ))
    }
}
