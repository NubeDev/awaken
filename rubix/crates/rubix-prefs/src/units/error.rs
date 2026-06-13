//! Errors emitted by the `units` module surface.

use thiserror::Error;

use super::{Quantity, Unit};

/// Failure modes for parsing / conversion in the units module.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UnitError {
    /// Wire form did not match any [`Quantity`] variant.
    #[error("unknown quantity code: {0:?}")]
    UnknownQuantity(String),
    /// Wire form did not match any [`Unit`] variant.
    #[error("unknown unit code: {0:?}")]
    UnknownUnit(String),
    /// The unit is known but not registered against the quantity
    /// (e.g. `Quantity::Temperature` + `Unit::Pound`).
    #[error("unit {unit} is not valid for quantity {quantity}")]
    UnitNotInQuantity {
        /// The quantity the caller asked about.
        quantity: Quantity,
        /// The unit the caller tried to use.
        unit: Unit,
    },
}
