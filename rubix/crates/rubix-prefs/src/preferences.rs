//! A user's display preferences: unit system + datetime pattern.
//!
//! The two display preferences bundled into the per-user setting the transport
//! layer applies to every response DTO (`rubix/docs/SCOPE.md`, "Preferences").
//! Both default to the canonical form (metric, ISO-8601), so an absent or
//! unconfigured preference renders values exactly as stored.

use serde::{Deserialize, Serialize};

use crate::datetime::DateTimePattern;
use crate::units::UnitSystem;

/// A user's display preferences.
///
/// Neither preference affects storage — they are applied at the response DTO
/// layer by [`apply_to`](crate::apply_to). The default is the canonical display
/// (metric units, ISO-8601 datetimes), so the transport layer can always build a
/// `Preferences` even for a user who set nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Preferences {
    /// The unit system tagged numeric fields are rendered in.
    #[serde(default)]
    pub units: UnitSystem,
    /// The strftime pattern timestamp fields are rendered with.
    #[serde(default)]
    pub datetime: DateTimePattern,
}

impl Preferences {
    /// Build preferences from a unit system and datetime pattern.
    #[must_use]
    pub fn new(units: UnitSystem, datetime: DateTimePattern) -> Self {
        Self { units, datetime }
    }
}
