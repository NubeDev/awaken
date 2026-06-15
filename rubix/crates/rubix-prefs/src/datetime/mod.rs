//! Per-user datetime display formatting.
//!
//! One of the two display preferences (`rubix/docs/SCOPE.md`, "Preferences").
//! Timestamps are stored canonically (RFC 3339, UTC); the [`DateTimePattern`]
//! preference decides the strftime pattern they are rendered with by [`format`].
//! Only the response DTO is affected — storage stays canonical.

mod format;
mod pattern;

pub use format::{format, format_json};
pub use pattern::DateTimePattern;
