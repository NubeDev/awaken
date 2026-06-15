//! Render a canonical timestamp in the user's datetime pattern.
//!
//! The one verb the datetime preference performs: parse a stored RFC 3339
//! instant and re-render it with the user's [`DateTimePattern`]
//! (`rubix/docs/SCOPE.md`, "Preferences"). Parsing fails closed — a string that
//! is not a recognised instant is reported, never emitted unchanged and
//! mislabelled as formatted.

use chrono::DateTime;
use serde_json::Value;

use crate::error::{PrefsError, Result};

use super::pattern::DateTimePattern;

/// Format the RFC 3339 instant `instant` using `pattern`.
///
/// The instant is interpreted in UTC (the canonical storage zone) and rendered
/// with the user's strftime pattern.
///
/// # Errors
/// Returns [`PrefsError::Timestamp`] if `instant` is not a valid RFC 3339 string.
pub fn format(instant: &str, pattern: &DateTimePattern) -> Result<String> {
    let parsed = DateTime::parse_from_rfc3339(instant)
        .map_err(|e| PrefsError::Timestamp(format!("{instant}: {e}")))?;
    Ok(parsed
        .with_timezone(&chrono::Utc)
        .format(pattern.as_str())
        .to_string())
}

/// Format a JSON `value` holding an RFC 3339 timestamp string using `pattern`.
///
/// The DTO layer carries timestamps as JSON strings; this adapts [`format`] to
/// that shape.
///
/// # Errors
/// Returns [`PrefsError::Timestamp`] if `value` is not a JSON string or not a
/// valid RFC 3339 instant.
pub fn format_json(value: &Value, pattern: &DateTimePattern) -> Result<Value> {
    let instant = value
        .as_str()
        .ok_or_else(|| PrefsError::Timestamp(format!("expected a string, got {value}")))?;
    Ok(Value::String(format(instant, pattern)?))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{DateTimePattern, format, format_json};

    #[test]
    fn renders_per_the_users_pattern() {
        let pattern = DateTimePattern::new("%d/%m/%Y");
        let out = format("2026-06-15T09:30:00Z", &pattern).expect("format");
        assert_eq!(out, "15/06/2026");
    }

    #[test]
    fn default_pattern_is_iso_like() {
        let out = format("2026-06-15T09:30:00Z", &DateTimePattern::default()).expect("format");
        assert_eq!(out, "2026-06-15 09:30:00");
    }

    #[test]
    fn offset_is_normalised_to_utc() {
        let out =
            format("2026-06-15T11:30:00+02:00", &DateTimePattern::default()).expect("format");
        assert_eq!(out, "2026-06-15 09:30:00");
    }

    #[test]
    fn non_rfc3339_is_rejected() {
        assert!(format("not-a-date", &DateTimePattern::default()).is_err());
        assert!(format_json(&json!(42), &DateTimePattern::default()).is_err());
    }
}
