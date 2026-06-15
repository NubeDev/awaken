//! A user's display preferences: unit system + datetime pattern + timezone.
//!
//! The display preferences bundled into the per-user setting the transport layer
//! applies (`rubix/docs/SCOPE.md`, "Preferences"; `DASHBOARDS-SCOPE.md` §2). Units
//! and the datetime pattern default to the canonical form (metric, ISO-8601), so
//! an absent or unconfigured preference renders values exactly as stored. The
//! **timezone** is an IANA name a chart formats its UTC instants in on the client
//! (UTC instant + tz + pattern → label); an absent timezone means UTC.

use serde::{Deserialize, Serialize};

use crate::datetime::DateTimePattern;
use crate::units::UnitSystem;

/// A user's display preferences.
///
/// None of these affect storage — they are applied at the response/display layer
/// (units server-side by [`apply_to`](crate::apply_to); datetime + timezone on the
/// client for charts, §2). The default is the canonical display (metric units,
/// ISO-8601 datetimes, UTC), so the transport layer can always build a
/// `Preferences` even for a user who set nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Preferences {
    /// The unit system tagged numeric fields are rendered in.
    #[serde(default)]
    pub units: UnitSystem,
    /// The strftime pattern timestamp fields are rendered with.
    #[serde(default)]
    pub datetime: DateTimePattern,
    /// The IANA timezone name a chart formats its UTC instants in (e.g.
    /// `"Australia/Sydney"`). `None` means UTC — the canonical display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

impl Preferences {
    /// Build preferences from a unit system and datetime pattern, UTC timezone.
    #[must_use]
    pub fn new(units: UnitSystem, datetime: DateTimePattern) -> Self {
        Self {
            units,
            datetime,
            timezone: None,
        }
    }

    /// Set the IANA timezone, returning the updated preferences (builder style).
    #[must_use]
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }
}
