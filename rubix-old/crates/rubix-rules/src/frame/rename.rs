//! `rename(from, to)` — rename one column, keeping all others.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Rename column `from` to `to`. Row count is unchanged.
    pub fn rename(&self, from: &str, to: &str) -> Result<Frame, RuleError> {
        require_column(self, from)?;
        let list = self
            .schema()
            .fields()
            .iter()
            .map(|field| {
                let name = field.name();
                if name == from {
                    format!("{} AS {}", quote_ident(from), quote_ident(to))
                } else {
                    quote_ident(name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.compute(&format!("SELECT {list} FROM {TABLE}"))
    }
}
