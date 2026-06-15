//! Per-user units: metric ↔ imperial display conversion.
//!
//! One of the two display preferences (`rubix/docs/SCOPE.md`, "Preferences").
//! Values are stored canonically (metric); the [`UnitSystem`] preference decides
//! whether a tagged [`Quantity`] is converted for display by [`convert`]. Only
//! the response DTO is affected — storage stays canonical.

mod convert;
mod quantity;
mod system;

pub use convert::{convert, convert_json};
pub use quantity::Quantity;
pub use system::UnitSystem;
