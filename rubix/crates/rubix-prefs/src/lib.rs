//! Per-user display preferences for the rubix platform.
//!
//! The Preferences component (`rubix/docs/SCOPE.md`, "Preferences";
//! `rubix/STACK-DEISGN.md`, `rubix-prefs` row): per-user **units**
//! (metric/imperial) and **datetime** formatting, applied at the response DTO
//! layer by the transport crate. Values are stored canonically (metric, RFC 3339
//! UTC); these preferences only change how a response is *displayed*, never what
//! is stored — so the same record renders differently per user without a second
//! copy.
//!
//! Laid out one verb per file (`rubix/docs/FILE-LAYOUT.md`): [`units`] owns the
//! metric↔imperial conversion, [`datetime`] the per-pattern timestamp rendering,
//! and [`apply_to`] the DTO rewrite the transport layer invokes.

mod apply;
mod datetime;
mod error;
mod preferences;
mod units;

pub use apply::{FieldSpec, apply_to};
pub use datetime::{DateTimePattern, format};
pub use error::{PrefsError, Result};
pub use preferences::Preferences;
pub use units::{Quantity, UnitSystem, convert, convert_json};
