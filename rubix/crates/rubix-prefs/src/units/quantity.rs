//! The physical quantity a numeric field carries.
//!
//! A bare number is dimensionless until it is tagged with the quantity it
//! measures; only then can a metric value be converted to imperial for display
//! (`rubix/docs/SCOPE.md`, "Preferences"). A response DTO names which of its
//! fields carry which [`Quantity`], so [`convert`](super::convert) knows the
//! conversion factor and offset to apply.

/// A physical quantity a numeric value measures.
///
/// The set is deliberately small — the quantities the dashboards display today.
/// Adding one is a new variant plus its factor/offset in
/// [`to_imperial`](Quantity::to_imperial), the single place the conversion math
/// lives so metric→imperial and the round-trip cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// Temperature in degrees Celsius (canonical) ↔ Fahrenheit (imperial).
    Temperature,
    /// Length in metres (canonical) ↔ feet (imperial).
    Length,
    /// Mass in kilograms (canonical) ↔ pounds (imperial).
    Mass,
    /// Speed in metres per second (canonical) ↔ miles per hour (imperial).
    Speed,
}

/// Feet per metre.
const FEET_PER_METRE: f64 = 3.280_839_895;
/// Pounds per kilogram.
const POUNDS_PER_KILOGRAM: f64 = 2.204_622_622;
/// Miles per hour per metre-per-second.
const MPH_PER_MPS: f64 = 2.236_936_292;

impl Quantity {
    /// Resolve a wire string to a quantity, or `None` if unknown.
    ///
    /// The strings are the lowercase variant names a chart's per-series
    /// `quantity` carries (`"temperature"`, `"length"`, `"mass"`, `"speed"`), so
    /// the query endpoint can map a column to its quantity from the chart config.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Quantity> {
        match raw {
            "temperature" => Some(Quantity::Temperature),
            "length" => Some(Quantity::Length),
            "mass" => Some(Quantity::Mass),
            "speed" => Some(Quantity::Speed),
            _ => None,
        }
    }

    /// Convert a canonical (metric) value of this quantity to imperial.
    #[must_use]
    pub fn to_imperial(self, metric: f64) -> f64 {
        match self {
            Quantity::Temperature => metric * 9.0 / 5.0 + 32.0,
            Quantity::Length => metric * FEET_PER_METRE,
            Quantity::Mass => metric * POUNDS_PER_KILOGRAM,
            Quantity::Speed => metric * MPH_PER_MPS,
        }
    }

    /// Convert an imperial value of this quantity back to canonical (metric).
    ///
    /// The exact inverse of [`to_imperial`](Quantity::to_imperial) — the round
    /// trip is identity within floating-point tolerance, which the unit tests
    /// assert.
    #[must_use]
    pub fn to_metric(self, imperial: f64) -> f64 {
        match self {
            Quantity::Temperature => (imperial - 32.0) * 5.0 / 9.0,
            Quantity::Length => imperial / FEET_PER_METRE,
            Quantity::Mass => imperial / POUNDS_PER_KILOGRAM,
            Quantity::Speed => imperial / MPH_PER_MPS,
        }
    }
}
