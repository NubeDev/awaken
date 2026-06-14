//! `anomalies(col, z_threshold)` — flag rows whose value is an outlier.
//!
//! Computes the z-score of `col` over the frame and appends a boolean
//! `<col>_anomaly` column that is true where `|z| >= z_threshold`. One value per
//! row, so the frame never grows. This is the "is this point an outlier" the
//! decision logic reads — e.g. `df.anomalies("kw", 3.0)` then a rule checks
//! whether any row flagged.

use super::compute::{quote_ident, require_column};
use super::{Frame, TABLE};
use crate::error::RuleError;

impl Frame {
    /// Append `<col>_anomaly`: true where `col`'s z-score magnitude ≥ threshold.
    pub fn anomalies(&self, col: &str, z_threshold: f64) -> Result<Frame, RuleError> {
        require_column(self, col)?;
        if !z_threshold.is_finite() || z_threshold < 0.0 {
            return Err(RuleError::Runtime(format!(
                "anomalies: z_threshold must be finite and >= 0, got {z_threshold}"
            )));
        }
        let cq = quote_ident(col);
        let out = quote_ident(&format!("{col}_anomaly"));
        let thr = format!("{z_threshold:?}");
        let z = format!(
            "CASE WHEN stddev({cq}) OVER () IS NULL \
                  OR stddev({cq}) OVER () = 0 THEN 0.0 \
             ELSE abs(({cq} - avg({cq}) OVER ()) / (stddev({cq}) OVER ())) END"
        );
        self.compute(&format!(
            "SELECT *, ({z}) >= {thr} AS {out} FROM {TABLE}"
        ))
    }
}
