//! `PreferencesPatch` — the wire shape of a `PATCH` body.
//!
//! Every field is `Option<T>`. The route layer in Phase 1 interprets
//! a missing key as "leave alone" and an explicit JSON `null` as
//! "revert to inherit"; serde collapses both to `None` here, so the
//! route layer reaches for `serde_json::Value` (or a custom
//! deserialiser) to disambiguate. This crate just carries the shape.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart};
use crate::units::Unit;

/// `PATCH` body shape — mirror of [`super::ResolvedPreferences`] with
/// every field made optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct PreferencesPatch {
    /// Optional IANA timezone identifier or `"auto"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,

    /// Optional BCP-47 locale tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    /// Optional BCP-47 language subtag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Optional `metric` / `imperial` toggle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_system: Option<UnitSystem>,

    /// Optional temperature unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_unit: Option<Unit>,
    /// Optional pressure unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure_unit: Option<Unit>,
    /// Optional speed unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_unit: Option<Unit>,
    /// Optional length unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_unit: Option<Unit>,
    /// Optional mass unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mass_unit: Option<Unit>,

    /// Optional date format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_format: Option<DateFormat>,
    /// Optional time format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<TimeFormat>,
    /// Optional week-start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub week_start: Option<WeekStart>,
    /// Optional number format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_format: Option<NumberFormat>,

    /// Optional ISO 4217 currency code or `"auto"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    /// Optional theme. User-only field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
}
