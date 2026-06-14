//! `UnitSystem` — `metric` vs `imperial`. Drives the `Auto` derivation
//! for per-unit fields on `ResolvedPreferences`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Closed enum of unit systems. Variants locked in stage 1 of the
/// Phase 0 plan. *Revisit trigger:* none expected — US-customary vs
/// Imperial nuance is handled in the per-unit derivation, not by
/// adding a third variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnitSystem {
    /// SI / metric system. Default for new users when ICU has no
    /// region-specific preference.
    Metric,
    /// Imperial system. Drives `Fahrenheit` / `MilePerHour` / `Foot` /
    /// `Pound` / `Psi` for per-unit `Auto` derivation.
    Imperial,
}
