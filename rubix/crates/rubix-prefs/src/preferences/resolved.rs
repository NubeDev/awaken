//! `ResolvedPreferences` — the post-resolver, fully-concrete view of
//! a principal's effective preferences.
//!
//! Per R3 of `DOCS/user/scope/SCOPE.md`:
//!
//! > The resolver is the only thing that produces a
//! > `ResolvedPreferences`. The DTO has no `Option` fields and no
//! > `"auto"` placeholder string — every NULL or `"auto"` from the
//! > user/org rows is concretised against the system default + ICU
//! > before this struct is constructed.
//!
//! Request handlers consume this directly; they never see the
//! resolution machinery.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart};
use crate::units::Unit;

/// Fully-resolved per-principal preferences. All fields are concrete —
/// the resolver has already collapsed user → org → system → ICU
/// chains, so callers can trust every value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct ResolvedPreferences {
    /// IANA timezone identifier (e.g. `"Europe/Paris"`,
    /// `"America/New_York"`). Never `"auto"` once resolved.
    pub timezone: String,

    /// BCP-47 locale tag (e.g. `"en-US"`, `"fr-FR"`). Drives ICU
    /// formatting fallbacks.
    pub locale: String,

    /// BCP-47 language subtag (e.g. `"en"`, `"fr"`). Drives i18n
    /// message catalogue lookup.
    pub language: String,

    /// `metric` or `imperial`. Drives per-unit `Auto` derivation
    /// upstream; by the time it lands here the per-unit fields below
    /// are already concrete, so this value is informational.
    pub unit_system: UnitSystem,

    /// Concrete temperature unit (one of `Celsius`, `Fahrenheit`).
    pub temperature_unit: Unit,
    /// Concrete pressure unit (one of `Kilopascal`, `Psi`, `Bar`).
    pub pressure_unit: Unit,
    /// Concrete speed unit (one of `MeterPerSecond`,
    /// `KilometerPerHour`, `MilePerHour`, `Knot`).
    pub speed_unit: Unit,
    /// Concrete length unit (one of `Meter`, `Foot`).
    pub length_unit: Unit,
    /// Concrete mass unit (one of `Kilogram`, `Pound`).
    pub mass_unit: Unit,

    /// Concrete date format — never `Auto` once resolved.
    pub date_format: DateFormat,
    /// Concrete time format — never `Auto` once resolved.
    pub time_format: TimeFormat,
    /// Concrete week-start — never `Auto` once resolved.
    pub week_start: WeekStart,
    /// Concrete number format — never `Auto` once resolved.
    pub number_format: NumberFormat,

    /// ISO 4217 currency code (e.g. `"USD"`, `"EUR"`). Never `"auto"`
    /// once resolved.
    pub currency: String,

    /// UI theme (`Light`, `Dark`, or `System`). User-only field.
    pub theme: Theme,
}

