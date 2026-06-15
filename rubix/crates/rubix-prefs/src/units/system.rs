//! The unit system a user reads values in.
//!
//! Per-user units are one of the two display preferences (`rubix/docs/SCOPE.md`,
//! "Preferences"). The platform stores values in one canonical (metric) form;
//! this preference decides whether a value is *displayed* converted to imperial.
//! Storage is never affected — only the response DTO the user sees.

use serde::{Deserialize, Serialize};

/// Which unit system a user's display values are rendered in.
///
/// Metric is the canonical storage system, so a metric preference is a no-op
/// pass-through; imperial converts each tagged quantity at the DTO layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnitSystem {
    /// SI / metric units — the canonical storage form.
    #[default]
    Metric,
    /// US customary / imperial units, converted from the stored metric value.
    Imperial,
}
