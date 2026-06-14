//! `fill_null(strategy)` — replace nulls across numeric columns.
//!
//! Strategies: `"zero"` (replace with 0) and `"mean"` (replace with the column
//! mean over the frame). A scalar/window replacement never changes row count.
//! Only numeric columns are touched; others pass through unchanged.

use datafusion::arrow::datatypes::DataType;

use super::compute::quote_ident;
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Fill nulls in numeric columns per `strategy` (`"zero"` or `"mean"`).
    pub fn fill_null(&self, strategy: &str) -> Result<Frame, RuleError> {
        let projections = self
            .schema()
            .fields()
            .iter()
            .map(|field| {
                let name = field.name();
                let q = quote_ident(name);
                if !is_numeric(field.data_type()) {
                    return Ok(q);
                }
                let replacement = match strategy {
                    "zero" => "0".to_string(),
                    "mean" => format!("avg({q}) OVER ()"),
                    other => {
                        return Err(RuleError::Runtime(format!(
                            "fill_null: unknown strategy `{other}` (use zero or mean)"
                        )))
                    }
                };
                Ok(format!("coalesce({q}, {replacement}) AS {q}"))
            })
            .collect::<Result<Vec<_>, RuleError>>()?
            .join(", ");
        self.compute(&format!("SELECT {projections} FROM {TABLE}"))
    }
}

fn is_numeric(dt: &DataType) -> bool {
    dt.is_numeric()
}
