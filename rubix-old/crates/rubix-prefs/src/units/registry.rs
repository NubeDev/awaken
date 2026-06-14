//! `UnitRegistry` trait + `StaticRegistry` compile-time impl.
//!
//! Per R4 the registry is "the thin serialisable veneer; we never
//! hand-write conversion math." Look up the canonical unit + allowed
//! units for a quantity here; do the arithmetic via
//! [`super::normalize_for_storage`] (which delegates to `uom`).

use super::{Quantity, Unit};

/// Per-quantity metadata: the canonical SI unit and every unit the
/// platform will accept on the wire for that quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantityDef {
    /// Canonical SI unit. Storage is always in this unit (R1).
    pub canonical: Unit,
    /// Every unit accepted on the wire for this quantity, including
    /// the canonical. Backed by a `'static` slice so the registry
    /// stays compile-time and zero-alloc.
    pub allowed_units: &'static [Unit],
}

/// Read-only view of the quantity → unit table.
pub trait UnitRegistry {
    /// Definition for `quantity`, or `None` if the registry does not
    /// know the quantity. The closed enum makes `None` impossible for
    /// the built-in [`StaticRegistry`], but the trait keeps the door
    /// open for future dynamic registries.
    fn get(&self, quantity: Quantity) -> Option<&QuantityDef>;

    /// `true` iff `unit` is accepted for `quantity`.
    fn supports(&self, quantity: Quantity, unit: Unit) -> bool {
        self.get(quantity)
            .is_some_and(|def| def.allowed_units.contains(&unit))
    }
}

/// Compile-time registry — the single source of truth for the v1
/// surface. Cheap to clone (zero-sized) and shareable across the
/// process; consumers can hold one as a `&'static` if they prefer.
#[derive(Debug, Default, Clone, Copy)]
pub struct StaticRegistry;

impl StaticRegistry {
    /// Construct the registry. No-op — kept for API symmetry with
    /// future dynamic registries.
    pub const fn new() -> Self {
        Self
    }
}

const TEMPERATURE: QuantityDef = QuantityDef {
    canonical: Unit::Celsius,
    allowed_units: &[Unit::Celsius, Unit::Fahrenheit],
};

const PRESSURE: QuantityDef = QuantityDef {
    canonical: Unit::Kilopascal,
    allowed_units: &[Unit::Kilopascal, Unit::Psi, Unit::Bar],
};

const SPEED: QuantityDef = QuantityDef {
    canonical: Unit::MeterPerSecond,
    allowed_units: &[
        Unit::MeterPerSecond,
        Unit::KilometerPerHour,
        Unit::MilePerHour,
        Unit::Knot,
    ],
};

const LENGTH: QuantityDef = QuantityDef {
    canonical: Unit::Meter,
    allowed_units: &[Unit::Meter, Unit::Foot],
};

const MASS: QuantityDef = QuantityDef {
    canonical: Unit::Kilogram,
    allowed_units: &[Unit::Kilogram, Unit::Pound],
};

const DURATION: QuantityDef = QuantityDef {
    canonical: Unit::Second,
    allowed_units: &[
        Unit::Second,
        Unit::Millisecond,
        Unit::Minute,
        Unit::Hour,
        Unit::Day,
    ],
};

const VOLUME: QuantityDef = QuantityDef {
    canonical: Unit::CubicMeter,
    allowed_units: &[
        Unit::CubicMeter,
        Unit::Liter,
        Unit::Milliliter,
        Unit::GallonUs,
        Unit::FluidOunceUs,
    ],
};

const ENERGY: QuantityDef = QuantityDef {
    canonical: Unit::Joule,
    allowed_units: &[Unit::Joule, Unit::Kilojoule, Unit::KilowattHour, Unit::Btu],
};

const POWER: QuantityDef = QuantityDef {
    canonical: Unit::Watt,
    allowed_units: &[Unit::Watt, Unit::Kilowatt, Unit::Horsepower],
};

const AREA: QuantityDef = QuantityDef {
    canonical: Unit::SquareMeter,
    allowed_units: &[
        Unit::SquareMeter,
        Unit::SquareFoot,
        Unit::Acre,
        Unit::Hectare,
    ],
};

const ANGLE: QuantityDef = QuantityDef {
    canonical: Unit::Radian,
    allowed_units: &[Unit::Radian, Unit::Degree],
};

const FREQUENCY: QuantityDef = QuantityDef {
    canonical: Unit::Hertz,
    allowed_units: &[Unit::Hertz, Unit::Kilohertz, Unit::Megahertz],
};

impl UnitRegistry for StaticRegistry {
    fn get(&self, quantity: Quantity) -> Option<&QuantityDef> {
        Some(match quantity {
            Quantity::Temperature => &TEMPERATURE,
            Quantity::Pressure => &PRESSURE,
            Quantity::Speed => &SPEED,
            Quantity::Length => &LENGTH,
            Quantity::Mass => &MASS,
            Quantity::Duration => &DURATION,
            Quantity::Volume => &VOLUME,
            Quantity::Energy => &ENERGY,
            Quantity::Power => &POWER,
            Quantity::Area => &AREA,
            Quantity::Angle => &ANGLE,
            Quantity::Frequency => &FREQUENCY,
        })
    }
}
