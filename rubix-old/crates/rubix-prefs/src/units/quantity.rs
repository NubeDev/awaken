//! `Quantity` — the closed enum of physical quantities `starter`
//! knows about.
//!
//! # R4 (verbatim)
//!
//! > `starter-spi` owns the `Quantity` and `Unit` enums and the
//! > `UnitRegistry` trait + `StaticRegistry` impl. The enums are
//! > **closed** — extensions cannot add variants — because every wire
//! > identifier and every UI label must be known to the platform. New
//! > quantities or units land via PR on `starter-spi`; the friction is
//! > intentional and matches workspace R8 (small public surface, slow
//! > changes).
//!
//! Per R4 the type is intentionally **not** `#[non_exhaustive]` — that
//! attribute would defeat the closed-enum guarantee callers rely on.
//! New variants are a deliberate PR that bumps the surface.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

use super::UnitError;

/// Closed enum of physical quantities the platform recognises.
///
/// See module docs for the R4 quote that pins this type's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Quantity {
    /// Thermodynamic temperature. Canonical unit: degree Celsius.
    Temperature,
    /// Pressure. Canonical unit: kilopascal.
    Pressure,
    /// Linear speed. Canonical unit: metre per second.
    Speed,
    /// Length. Canonical unit: metre.
    Length,
    /// Mass. Canonical unit: kilogram.
    Mass,
    /// Time / duration. Canonical unit: second.
    Duration,
    /// Volume. Canonical unit: cubic metre.
    Volume,
    /// Energy. Canonical unit: joule.
    Energy,
    /// Power. Canonical unit: watt.
    Power,
    /// Area. Canonical unit: square metre.
    Area,
    /// Plane angle. Canonical unit: radian.
    Angle,
    /// Frequency. Canonical unit: hertz.
    Frequency,
}

impl Quantity {
    /// Lowercase wire identifier — matches the SCOPE Per-series unit
    /// metadata examples (`"temperature"`, …).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Temperature => "temperature",
            Self::Pressure => "pressure",
            Self::Speed => "speed",
            Self::Length => "length",
            Self::Mass => "mass",
            Self::Duration => "duration",
            Self::Volume => "volume",
            Self::Energy => "energy",
            Self::Power => "power",
            Self::Area => "area",
            Self::Angle => "angle",
            Self::Frequency => "frequency",
        }
    }

    /// Every variant in declaration order. Lets the registry build
    /// itself without `strum`.
    pub const ALL: &'static [Quantity] = &[
        Quantity::Temperature,
        Quantity::Pressure,
        Quantity::Speed,
        Quantity::Length,
        Quantity::Mass,
        Quantity::Duration,
        Quantity::Volume,
        Quantity::Energy,
        Quantity::Power,
        Quantity::Area,
        Quantity::Angle,
        Quantity::Frequency,
    ];
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Quantity {
    type Err = UnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "temperature" => Ok(Self::Temperature),
            "pressure" => Ok(Self::Pressure),
            "speed" => Ok(Self::Speed),
            "length" => Ok(Self::Length),
            "mass" => Ok(Self::Mass),
            "duration" => Ok(Self::Duration),
            "volume" => Ok(Self::Volume),
            "energy" => Ok(Self::Energy),
            "power" => Ok(Self::Power),
            "area" => Ok(Self::Area),
            "angle" => Ok(Self::Angle),
            "frequency" => Ok(Self::Frequency),
            other => Err(UnitError::UnknownQuantity(other.to_owned())),
        }
    }
}
