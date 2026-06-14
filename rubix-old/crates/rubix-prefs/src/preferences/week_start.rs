//! `WeekStart` — `monday`, `sunday`, or `auto`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Closed enum of week-start choices. Variants locked in stage 1.
/// *Revisit trigger:* a locale that starts the week on Saturday (parts
/// of MENA) surfaces as a shipped requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeekStart {
    /// Defer to ICU's `firstDayOfWeek` for the active locale.
    Auto,
    /// Week starts on Monday.
    Monday,
    /// Week starts on Sunday.
    Sunday,
}
