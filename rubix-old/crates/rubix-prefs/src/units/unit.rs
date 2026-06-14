//! `Unit` — the closed enum of unit codes the platform recognises.
//!
//! The wire form is the lowercase, underscore-joined name (e.g.
//! `"fahrenheit"`, `"meter_per_second"`) per the SCOPE Per-series unit
//! metadata example. The same identifier is what `GET /v1/units`
//! exposes to clients. See [`super::quantity::Quantity`] for the R4
//! verbatim quote on why this enum is closed.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

use super::UnitError;

/// Closed enum of unit codes for the v1 surface. Variants are locked
/// in stage 1 of the Phase 0 plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    // -- Temperature ----------------------------------------------------
    /// Degree Celsius — canonical unit for `Quantity::Temperature`.
    Celsius,
    /// Degree Fahrenheit.
    Fahrenheit,

    // -- Pressure -------------------------------------------------------
    /// Kilopascal — canonical unit for `Quantity::Pressure`.
    Kilopascal,
    /// Pound-force per square inch.
    Psi,
    /// Bar.
    Bar,

    // -- Speed ----------------------------------------------------------
    /// Metre per second — canonical unit for `Quantity::Speed`.
    MeterPerSecond,
    /// Kilometre per hour.
    KilometerPerHour,
    /// Mile per hour.
    MilePerHour,
    /// Knot (nautical mile per hour).
    Knot,

    // -- Length ---------------------------------------------------------
    /// Metre — canonical unit for `Quantity::Length`.
    Meter,
    /// International foot.
    Foot,

    // -- Mass -----------------------------------------------------------
    /// Kilogram — canonical unit for `Quantity::Mass`.
    Kilogram,
    /// International avoirdupois pound.
    Pound,

    // -- Duration -------------------------------------------------------
    /// Second — canonical unit for `Quantity::Duration`.
    Second,
    /// Millisecond.
    Millisecond,
    /// Minute (60 s).
    Minute,
    /// Hour (3600 s).
    Hour,
    /// Day (86 400 s).
    Day,

    // -- Volume ---------------------------------------------------------
    /// Cubic metre — canonical unit for `Quantity::Volume`.
    CubicMeter,
    /// Litre.
    Liter,
    /// Millilitre.
    Milliliter,
    /// US liquid gallon.
    GallonUs,
    /// US fluid ounce.
    FluidOunceUs,

    // -- Energy ---------------------------------------------------------
    /// Joule — canonical unit for `Quantity::Energy`.
    Joule,
    /// Kilojoule.
    Kilojoule,
    /// Kilowatt-hour.
    KilowattHour,
    /// International table BTU.
    Btu,

    // -- Power ----------------------------------------------------------
    /// Watt — canonical unit for `Quantity::Power`.
    Watt,
    /// Kilowatt.
    Kilowatt,
    /// Mechanical horsepower.
    Horsepower,

    // -- Area -----------------------------------------------------------
    /// Square metre — canonical unit for `Quantity::Area`.
    SquareMeter,
    /// Square foot.
    SquareFoot,
    /// Acre.
    Acre,
    /// Hectare.
    Hectare,

    // -- Angle ----------------------------------------------------------
    /// Radian — canonical unit for `Quantity::Angle`.
    Radian,
    /// Degree.
    Degree,

    // -- Frequency ------------------------------------------------------
    /// Hertz — canonical unit for `Quantity::Frequency`.
    Hertz,
    /// Kilohertz.
    Kilohertz,
    /// Megahertz.
    Megahertz,
}

impl Unit {
    /// Lowercase wire identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Celsius => "celsius",
            Self::Fahrenheit => "fahrenheit",
            Self::Kilopascal => "kilopascal",
            Self::Psi => "psi",
            Self::Bar => "bar",
            Self::MeterPerSecond => "meter_per_second",
            Self::KilometerPerHour => "kilometer_per_hour",
            Self::MilePerHour => "mile_per_hour",
            Self::Knot => "knot",
            Self::Meter => "meter",
            Self::Foot => "foot",
            Self::Kilogram => "kilogram",
            Self::Pound => "pound",
            Self::Second => "second",
            Self::Millisecond => "millisecond",
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::Day => "day",
            Self::CubicMeter => "cubic_meter",
            Self::Liter => "liter",
            Self::Milliliter => "milliliter",
            Self::GallonUs => "gallon_us",
            Self::FluidOunceUs => "fluid_ounce_us",
            Self::Joule => "joule",
            Self::Kilojoule => "kilojoule",
            Self::KilowattHour => "kilowatt_hour",
            Self::Btu => "btu",
            Self::Watt => "watt",
            Self::Kilowatt => "kilowatt",
            Self::Horsepower => "horsepower",
            Self::SquareMeter => "square_meter",
            Self::SquareFoot => "square_foot",
            Self::Acre => "acre",
            Self::Hectare => "hectare",
            Self::Radian => "radian",
            Self::Degree => "degree",
            Self::Hertz => "hertz",
            Self::Kilohertz => "kilohertz",
            Self::Megahertz => "megahertz",
        }
    }

    /// Short display symbol (e.g. `°C`, `m/s`, `kWh`). Used by
    /// [`super::convert_for_display`] so callers don't have to
    /// hand-maintain a symbol table at the presentation edge.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Celsius => "°C",
            Self::Fahrenheit => "°F",
            Self::Kilopascal => "kPa",
            Self::Psi => "psi",
            Self::Bar => "bar",
            Self::MeterPerSecond => "m/s",
            Self::KilometerPerHour => "km/h",
            Self::MilePerHour => "mph",
            Self::Knot => "kn",
            Self::Meter => "m",
            Self::Foot => "ft",
            Self::Kilogram => "kg",
            Self::Pound => "lb",
            Self::Second => "s",
            Self::Millisecond => "ms",
            Self::Minute => "min",
            Self::Hour => "h",
            Self::Day => "d",
            Self::CubicMeter => "m³",
            Self::Liter => "L",
            Self::Milliliter => "mL",
            Self::GallonUs => "gal",
            Self::FluidOunceUs => "fl oz",
            Self::Joule => "J",
            Self::Kilojoule => "kJ",
            Self::KilowattHour => "kWh",
            Self::Btu => "BTU",
            Self::Watt => "W",
            Self::Kilowatt => "kW",
            Self::Horsepower => "hp",
            Self::SquareMeter => "m²",
            Self::SquareFoot => "ft²",
            Self::Acre => "ac",
            Self::Hectare => "ha",
            Self::Radian => "rad",
            Self::Degree => "°",
            Self::Hertz => "Hz",
            Self::Kilohertz => "kHz",
            Self::Megahertz => "MHz",
        }
    }

    /// Every variant in declaration order.
    pub const ALL: &'static [Unit] = &[
        Unit::Celsius,
        Unit::Fahrenheit,
        Unit::Kilopascal,
        Unit::Psi,
        Unit::Bar,
        Unit::MeterPerSecond,
        Unit::KilometerPerHour,
        Unit::MilePerHour,
        Unit::Knot,
        Unit::Meter,
        Unit::Foot,
        Unit::Kilogram,
        Unit::Pound,
        Unit::Second,
        Unit::Millisecond,
        Unit::Minute,
        Unit::Hour,
        Unit::Day,
        Unit::CubicMeter,
        Unit::Liter,
        Unit::Milliliter,
        Unit::GallonUs,
        Unit::FluidOunceUs,
        Unit::Joule,
        Unit::Kilojoule,
        Unit::KilowattHour,
        Unit::Btu,
        Unit::Watt,
        Unit::Kilowatt,
        Unit::Horsepower,
        Unit::SquareMeter,
        Unit::SquareFoot,
        Unit::Acre,
        Unit::Hectare,
        Unit::Radian,
        Unit::Degree,
        Unit::Hertz,
        Unit::Kilohertz,
        Unit::Megahertz,
    ];
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Unit {
    type Err = UnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "celsius" => Ok(Self::Celsius),
            "fahrenheit" => Ok(Self::Fahrenheit),
            "kilopascal" => Ok(Self::Kilopascal),
            "psi" => Ok(Self::Psi),
            "bar" => Ok(Self::Bar),
            "meter_per_second" => Ok(Self::MeterPerSecond),
            "kilometer_per_hour" => Ok(Self::KilometerPerHour),
            "mile_per_hour" => Ok(Self::MilePerHour),
            "knot" => Ok(Self::Knot),
            "meter" => Ok(Self::Meter),
            "foot" => Ok(Self::Foot),
            "kilogram" => Ok(Self::Kilogram),
            "pound" => Ok(Self::Pound),
            "second" => Ok(Self::Second),
            "millisecond" => Ok(Self::Millisecond),
            "minute" => Ok(Self::Minute),
            "hour" => Ok(Self::Hour),
            "day" => Ok(Self::Day),
            "cubic_meter" => Ok(Self::CubicMeter),
            "liter" => Ok(Self::Liter),
            "milliliter" => Ok(Self::Milliliter),
            "gallon_us" => Ok(Self::GallonUs),
            "fluid_ounce_us" => Ok(Self::FluidOunceUs),
            "joule" => Ok(Self::Joule),
            "kilojoule" => Ok(Self::Kilojoule),
            "kilowatt_hour" => Ok(Self::KilowattHour),
            "btu" => Ok(Self::Btu),
            "watt" => Ok(Self::Watt),
            "kilowatt" => Ok(Self::Kilowatt),
            "horsepower" => Ok(Self::Horsepower),
            "square_meter" => Ok(Self::SquareMeter),
            "square_foot" => Ok(Self::SquareFoot),
            "acre" => Ok(Self::Acre),
            "hectare" => Ok(Self::Hectare),
            "radian" => Ok(Self::Radian),
            "degree" => Ok(Self::Degree),
            "hertz" => Ok(Self::Hertz),
            "kilohertz" => Ok(Self::Kilohertz),
            "megahertz" => Ok(Self::Megahertz),
            other => Err(UnitError::UnknownUnit(other.to_owned())),
        }
    }
}
