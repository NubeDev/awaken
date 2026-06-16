//! Wire shapes for the per-principal preferences resource (§2).
//!
//! `rubix-prefs` defines the [`Preferences`] model (units, datetime pattern,
//! timezone); this is its transport DTO. `GET /prefs` returns the requesting
//! principal's stored preferences (defaults when none are set); `PATCH /prefs`
//! updates them. Preferences are stored as a principal-scoped `kind:"prefs"`
//! record so they ride the gate/audit like every other write
//! (`rubix/docs/design/DASHBOARDS-SCOPE.md` §2).
//!
//! Units cross the wire as the serde string (`"metric"`/`"imperial"`) rather than
//! the `rubix-prefs` enum so the DTO owns its own OpenAPI schema without deriving
//! `ToSchema` on a foreign type.

use rubix_prefs::{DateTimePattern, Preferences, UnitSystem};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The marker kind for a preferences record's content.
pub const PREFS_KIND: &str = "prefs";

/// A principal's preferences as returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PreferencesDto {
    /// The unit system numeric values are displayed in (`metric` or `imperial`).
    pub units: String,
    /// The strftime pattern timestamps are displayed with.
    pub datetime: String,
    /// The IANA timezone a chart formats its UTC instants in (`null` = UTC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

impl From<Preferences> for PreferencesDto {
    fn from(prefs: Preferences) -> Self {
        Self {
            units: unit_system_str(prefs.units).to_owned(),
            datetime: prefs.datetime.as_str().to_owned(),
            timezone: prefs.timezone,
        }
    }
}

/// A partial update to a principal's preferences — every field optional, so a
/// client can change one preference without resending the others.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct UpdatePreferencesRequest {
    /// The new unit system (`metric`/`imperial`), if changing it.
    #[serde(default)]
    pub units: Option<String>,
    /// The new datetime pattern, if changing it.
    #[serde(default)]
    pub datetime: Option<String>,
    /// The new IANA timezone, if changing it.
    #[serde(default)]
    pub timezone: Option<String>,
}

impl UpdatePreferencesRequest {
    /// Apply this partial update onto `base`, leaving unset fields unchanged.
    ///
    /// # Errors
    /// Returns `Err` with the offending value if `units` is set but is neither
    /// `metric` nor `imperial`.
    pub fn merge_onto(self, mut base: Preferences) -> Result<Preferences, String> {
        if let Some(units) = self.units {
            base.units = parse_unit_system(&units)?;
        }
        if let Some(datetime) = self.datetime {
            base.datetime = DateTimePattern::new(datetime);
        }
        if let Some(timezone) = self.timezone {
            base.timezone = Some(timezone);
        }
        Ok(base)
    }
}

/// The serde string for a unit system.
fn unit_system_str(units: UnitSystem) -> &'static str {
    match units {
        UnitSystem::Metric => "metric",
        UnitSystem::Imperial => "imperial",
    }
}

/// Parse a unit-system string, rejecting anything but the two known systems.
fn parse_unit_system(raw: &str) -> Result<UnitSystem, String> {
    match raw {
        "metric" => Ok(UnitSystem::Metric),
        "imperial" => Ok(UnitSystem::Imperial),
        other => Err(format!("unknown unit system: {other}")),
    }
}
