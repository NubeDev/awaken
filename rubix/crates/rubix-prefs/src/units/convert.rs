//! Convert a stored metric value to the user's unit system.
//!
//! The one verb the unit preference performs: take a canonical (metric) number
//! and a [`Quantity`], and render it in the user's [`UnitSystem`]
//! (`rubix/docs/SCOPE.md`, "Preferences"). Metric is a pass-through (storage is
//! canonical); imperial applies the quantity's conversion. The conversion math
//! itself lives on [`Quantity`] so this verb stays a thin dispatch.

use serde_json::Value;

use crate::error::{PrefsError, Result};

use super::quantity::Quantity;
use super::system::UnitSystem;

/// Render `metric` (a canonical value of `quantity`) in `system`.
///
/// A metric preference returns the value unchanged; an imperial preference
/// converts it. The value must be a finite number.
///
/// # Errors
/// Returns [`PrefsError::NotNumeric`] if `metric` is not finite.
pub fn convert(metric: f64, quantity: Quantity, system: UnitSystem) -> Result<f64> {
    if !metric.is_finite() {
        return Err(PrefsError::NotNumeric);
    }
    Ok(match system {
        UnitSystem::Metric => metric,
        UnitSystem::Imperial => quantity.to_imperial(metric),
    })
}

/// Render a JSON `value` holding a canonical metric number in `system`.
///
/// The DTO layer carries values as JSON; this adapts [`convert`] to that shape,
/// rejecting a non-numeric JSON value rather than passing it through unconverted.
///
/// # Errors
/// Returns [`PrefsError::NotNumeric`] if `value` is not a finite JSON number.
pub fn convert_json(value: &Value, quantity: Quantity, system: UnitSystem) -> Result<Value> {
    let metric = value.as_f64().ok_or(PrefsError::NotNumeric)?;
    let rendered = convert(metric, quantity, system)?;
    serde_json::Number::from_f64(rendered)
        .map(Value::Number)
        .ok_or(PrefsError::NotNumeric)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Quantity, UnitSystem, convert, convert_json};

    #[test]
    fn metric_is_a_passthrough() {
        let value = convert(21.5, Quantity::Temperature, UnitSystem::Metric).expect("convert");
        assert!((value - 21.5).abs() < f64::EPSILON);
    }

    #[test]
    fn temperature_converts_to_fahrenheit() {
        let value = convert(100.0, Quantity::Temperature, UnitSystem::Imperial).expect("convert");
        assert!((value - 212.0).abs() < 1e-9);
    }

    #[test]
    fn metric_imperial_round_trips_for_every_quantity() {
        for quantity in [
            Quantity::Temperature,
            Quantity::Length,
            Quantity::Mass,
            Quantity::Speed,
        ] {
            let metric = 42.0_f64;
            let imperial = quantity.to_imperial(metric);
            let back = quantity.to_metric(imperial);
            assert!((back - metric).abs() < 1e-6, "round trip drifted for {quantity:?}");
        }
    }

    #[test]
    fn non_finite_is_rejected() {
        assert!(convert(f64::NAN, Quantity::Mass, UnitSystem::Imperial).is_err());
        assert!(convert(f64::INFINITY, Quantity::Mass, UnitSystem::Metric).is_err());
    }

    #[test]
    fn convert_json_rejects_a_non_number() {
        let err = convert_json(&json!("hot"), Quantity::Temperature, UnitSystem::Imperial);
        assert!(err.is_err());
    }
}
