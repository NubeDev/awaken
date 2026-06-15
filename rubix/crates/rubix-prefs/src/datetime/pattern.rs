//! The datetime display pattern a user reads timestamps in.
//!
//! The second display preference (`rubix/docs/SCOPE.md`, "Preferences").
//! Timestamps are stored canonically (RFC 3339, UTC); this preference decides
//! the strftime-style pattern they are *rendered* with for a given user. Storage
//! is never affected — only the response DTO.

use serde::{Deserialize, Serialize};

/// A user's datetime display pattern.
///
/// Holds a `chrono` strftime format string. The default is ISO-8601 date-time
/// (`%Y-%m-%d %H:%M:%S`), the unambiguous canonical display; a user may choose a
/// locale-style pattern instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateTimePattern(String);

impl DateTimePattern {
    /// Build a pattern from a `chrono` strftime format string.
    #[must_use]
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    /// The strftime format string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for DateTimePattern {
    fn default() -> Self {
        Self("%Y-%m-%d %H:%M:%S".to_owned())
    }
}
