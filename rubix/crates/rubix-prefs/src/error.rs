//! Preference-application errors.
//!
//! `rubix-prefs` formats a response DTO for display (`rubix/docs/SCOPE.md`,
//! "Preferences"). The only failure modes are a value that cannot be interpreted
//! as the physical quantity a conversion expects, or a timestamp that cannot be
//! parsed for re-formatting — both reported rather than silently passed through,
//! so a malformed value never reaches the wire mislabelled.

use thiserror::Error;

/// An error applying a preference transform to a response value.
#[derive(Debug, Error)]
pub enum PrefsError {
    /// A value tagged for unit conversion was not a finite number.
    #[error("value for unit conversion is not a finite number")]
    NotNumeric,
    /// A timestamp string could not be parsed for re-formatting.
    #[error("timestamp is not a recognised RFC 3339 instant: {0}")]
    Timestamp(String),
}

/// Result alias for preference application.
pub type Result<T> = std::result::Result<T, PrefsError>;
