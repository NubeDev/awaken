//! Conversion between wire units and canonical SI storage units.
//!
//! Per R4 conversion factors are delegated to `uom`; this module is
//! the only place in the workspace that names `uom` directly. Add a
//! new unit variant → add a match arm in both [`normalize_for_storage`]
//! (wire → canonical) and [`from_canonical`] (canonical → wire).

use uom::si::angle::{degree as angle_degree, radian};
use uom::si::area::{acre, hectare, square_foot, square_meter};
use uom::si::energy::{btu_it as energy_btu, joule, kilojoule, kilowatt_hour};
use uom::si::f64::{
    Angle as UomAngle, Area as UomArea, Energy as UomEnergy, Frequency as UomFrequency, Length,
    Mass, Power as UomPower, Pressure, ThermodynamicTemperature, Time as UomTime, Velocity,
    Volume as UomVolume,
};
use uom::si::frequency::{hertz, kilohertz, megahertz};
use uom::si::length::{foot, meter};
use uom::si::mass::{kilogram, pound};
use uom::si::power::{horsepower, kilowatt, watt};
use uom::si::pressure::{bar, kilopascal, psi};
use uom::si::thermodynamic_temperature::{degree_celsius, degree_fahrenheit};
use uom::si::time::{day, hour, millisecond, minute, second};
use uom::si::velocity::{kilometer_per_hour, knot, meter_per_second, mile_per_hour};
use uom::si::volume::{cubic_meter, fluid_ounce, gallon, liter, milliliter};

use super::{Quantity, Unit, UnitError};

/// Convert `value` (in `source_unit`) to the canonical SI unit for
/// `quantity`. Identity when `source_unit` is already canonical.
pub fn normalize_for_storage(
    quantity: Quantity,
    value: f64,
    source_unit: Unit,
) -> Result<f64, UnitError> {
    match (quantity, source_unit) {
        (Quantity::Temperature, Unit::Celsius) => {
            Ok(ThermodynamicTemperature::new::<degree_celsius>(value).get::<degree_celsius>())
        }
        (Quantity::Temperature, Unit::Fahrenheit) => {
            Ok(ThermodynamicTemperature::new::<degree_fahrenheit>(value).get::<degree_celsius>())
        }

        (Quantity::Pressure, Unit::Kilopascal) => {
            Ok(Pressure::new::<kilopascal>(value).get::<kilopascal>())
        }
        (Quantity::Pressure, Unit::Psi) => Ok(Pressure::new::<psi>(value).get::<kilopascal>()),
        (Quantity::Pressure, Unit::Bar) => Ok(Pressure::new::<bar>(value).get::<kilopascal>()),

        (Quantity::Speed, Unit::MeterPerSecond) => {
            Ok(Velocity::new::<meter_per_second>(value).get::<meter_per_second>())
        }
        (Quantity::Speed, Unit::KilometerPerHour) => {
            Ok(Velocity::new::<kilometer_per_hour>(value).get::<meter_per_second>())
        }
        (Quantity::Speed, Unit::MilePerHour) => {
            Ok(Velocity::new::<mile_per_hour>(value).get::<meter_per_second>())
        }
        (Quantity::Speed, Unit::Knot) => Ok(Velocity::new::<knot>(value).get::<meter_per_second>()),

        (Quantity::Length, Unit::Meter) => Ok(Length::new::<meter>(value).get::<meter>()),
        (Quantity::Length, Unit::Foot) => Ok(Length::new::<foot>(value).get::<meter>()),

        (Quantity::Mass, Unit::Kilogram) => Ok(Mass::new::<kilogram>(value).get::<kilogram>()),
        (Quantity::Mass, Unit::Pound) => Ok(Mass::new::<pound>(value).get::<kilogram>()),

        (Quantity::Duration, Unit::Second) => Ok(UomTime::new::<second>(value).get::<second>()),
        (Quantity::Duration, Unit::Millisecond) => {
            Ok(UomTime::new::<millisecond>(value).get::<second>())
        }
        (Quantity::Duration, Unit::Minute) => Ok(UomTime::new::<minute>(value).get::<second>()),
        (Quantity::Duration, Unit::Hour) => Ok(UomTime::new::<hour>(value).get::<second>()),
        (Quantity::Duration, Unit::Day) => Ok(UomTime::new::<day>(value).get::<second>()),

        (Quantity::Volume, Unit::CubicMeter) => {
            Ok(UomVolume::new::<cubic_meter>(value).get::<cubic_meter>())
        }
        (Quantity::Volume, Unit::Liter) => Ok(UomVolume::new::<liter>(value).get::<cubic_meter>()),
        (Quantity::Volume, Unit::Milliliter) => {
            Ok(UomVolume::new::<milliliter>(value).get::<cubic_meter>())
        }
        (Quantity::Volume, Unit::GallonUs) => {
            Ok(UomVolume::new::<gallon>(value).get::<cubic_meter>())
        }
        (Quantity::Volume, Unit::FluidOunceUs) => {
            Ok(UomVolume::new::<fluid_ounce>(value).get::<cubic_meter>())
        }

        (Quantity::Energy, Unit::Joule) => Ok(UomEnergy::new::<joule>(value).get::<joule>()),
        (Quantity::Energy, Unit::Kilojoule) => {
            Ok(UomEnergy::new::<kilojoule>(value).get::<joule>())
        }
        (Quantity::Energy, Unit::KilowattHour) => {
            Ok(UomEnergy::new::<kilowatt_hour>(value).get::<joule>())
        }
        (Quantity::Energy, Unit::Btu) => Ok(UomEnergy::new::<energy_btu>(value).get::<joule>()),

        (Quantity::Power, Unit::Watt) => Ok(UomPower::new::<watt>(value).get::<watt>()),
        (Quantity::Power, Unit::Kilowatt) => Ok(UomPower::new::<kilowatt>(value).get::<watt>()),
        (Quantity::Power, Unit::Horsepower) => Ok(UomPower::new::<horsepower>(value).get::<watt>()),

        (Quantity::Area, Unit::SquareMeter) => {
            Ok(UomArea::new::<square_meter>(value).get::<square_meter>())
        }
        (Quantity::Area, Unit::SquareFoot) => {
            Ok(UomArea::new::<square_foot>(value).get::<square_meter>())
        }
        (Quantity::Area, Unit::Acre) => Ok(UomArea::new::<acre>(value).get::<square_meter>()),
        (Quantity::Area, Unit::Hectare) => Ok(UomArea::new::<hectare>(value).get::<square_meter>()),

        (Quantity::Angle, Unit::Radian) => Ok(UomAngle::new::<radian>(value).get::<radian>()),
        (Quantity::Angle, Unit::Degree) => Ok(UomAngle::new::<angle_degree>(value).get::<radian>()),

        (Quantity::Frequency, Unit::Hertz) => Ok(UomFrequency::new::<hertz>(value).get::<hertz>()),
        (Quantity::Frequency, Unit::Kilohertz) => {
            Ok(UomFrequency::new::<kilohertz>(value).get::<hertz>())
        }
        (Quantity::Frequency, Unit::Megahertz) => {
            Ok(UomFrequency::new::<megahertz>(value).get::<hertz>())
        }

        (q, u) => Err(UnitError::UnitNotInQuantity {
            quantity: q,
            unit: u,
        }),
    }
}

/// Convert a canonical SI value to `target_unit` for display. Inverse
/// of [`normalize_for_storage`]; mirrors SCOPE R1's "convert at the
/// presentation edge" rule.
pub fn from_canonical(
    quantity: Quantity,
    canonical: f64,
    target_unit: Unit,
) -> Result<f64, UnitError> {
    match (quantity, target_unit) {
        (Quantity::Temperature, Unit::Celsius) => {
            Ok(ThermodynamicTemperature::new::<degree_celsius>(canonical).get::<degree_celsius>())
        }
        (Quantity::Temperature, Unit::Fahrenheit) => {
            Ok(ThermodynamicTemperature::new::<degree_celsius>(canonical)
                .get::<degree_fahrenheit>())
        }

        (Quantity::Pressure, Unit::Kilopascal) => {
            Ok(Pressure::new::<kilopascal>(canonical).get::<kilopascal>())
        }
        (Quantity::Pressure, Unit::Psi) => Ok(Pressure::new::<kilopascal>(canonical).get::<psi>()),
        (Quantity::Pressure, Unit::Bar) => Ok(Pressure::new::<kilopascal>(canonical).get::<bar>()),

        (Quantity::Speed, Unit::MeterPerSecond) => {
            Ok(Velocity::new::<meter_per_second>(canonical).get::<meter_per_second>())
        }
        (Quantity::Speed, Unit::KilometerPerHour) => {
            Ok(Velocity::new::<meter_per_second>(canonical).get::<kilometer_per_hour>())
        }
        (Quantity::Speed, Unit::MilePerHour) => {
            Ok(Velocity::new::<meter_per_second>(canonical).get::<mile_per_hour>())
        }
        (Quantity::Speed, Unit::Knot) => {
            Ok(Velocity::new::<meter_per_second>(canonical).get::<knot>())
        }

        (Quantity::Length, Unit::Meter) => Ok(Length::new::<meter>(canonical).get::<meter>()),
        (Quantity::Length, Unit::Foot) => Ok(Length::new::<meter>(canonical).get::<foot>()),

        (Quantity::Mass, Unit::Kilogram) => Ok(Mass::new::<kilogram>(canonical).get::<kilogram>()),
        (Quantity::Mass, Unit::Pound) => Ok(Mass::new::<kilogram>(canonical).get::<pound>()),

        (Quantity::Duration, Unit::Second) => Ok(UomTime::new::<second>(canonical).get::<second>()),
        (Quantity::Duration, Unit::Millisecond) => {
            Ok(UomTime::new::<second>(canonical).get::<millisecond>())
        }
        (Quantity::Duration, Unit::Minute) => Ok(UomTime::new::<second>(canonical).get::<minute>()),
        (Quantity::Duration, Unit::Hour) => Ok(UomTime::new::<second>(canonical).get::<hour>()),
        (Quantity::Duration, Unit::Day) => Ok(UomTime::new::<second>(canonical).get::<day>()),

        (Quantity::Volume, Unit::CubicMeter) => {
            Ok(UomVolume::new::<cubic_meter>(canonical).get::<cubic_meter>())
        }
        (Quantity::Volume, Unit::Liter) => {
            Ok(UomVolume::new::<cubic_meter>(canonical).get::<liter>())
        }
        (Quantity::Volume, Unit::Milliliter) => {
            Ok(UomVolume::new::<cubic_meter>(canonical).get::<milliliter>())
        }
        (Quantity::Volume, Unit::GallonUs) => {
            Ok(UomVolume::new::<cubic_meter>(canonical).get::<gallon>())
        }
        (Quantity::Volume, Unit::FluidOunceUs) => {
            Ok(UomVolume::new::<cubic_meter>(canonical).get::<fluid_ounce>())
        }

        (Quantity::Energy, Unit::Joule) => Ok(UomEnergy::new::<joule>(canonical).get::<joule>()),
        (Quantity::Energy, Unit::Kilojoule) => {
            Ok(UomEnergy::new::<joule>(canonical).get::<kilojoule>())
        }
        (Quantity::Energy, Unit::KilowattHour) => {
            Ok(UomEnergy::new::<joule>(canonical).get::<kilowatt_hour>())
        }
        (Quantity::Energy, Unit::Btu) => Ok(UomEnergy::new::<joule>(canonical).get::<energy_btu>()),

        (Quantity::Power, Unit::Watt) => Ok(UomPower::new::<watt>(canonical).get::<watt>()),
        (Quantity::Power, Unit::Kilowatt) => Ok(UomPower::new::<watt>(canonical).get::<kilowatt>()),
        (Quantity::Power, Unit::Horsepower) => {
            Ok(UomPower::new::<watt>(canonical).get::<horsepower>())
        }

        (Quantity::Area, Unit::SquareMeter) => {
            Ok(UomArea::new::<square_meter>(canonical).get::<square_meter>())
        }
        (Quantity::Area, Unit::SquareFoot) => {
            Ok(UomArea::new::<square_meter>(canonical).get::<square_foot>())
        }
        (Quantity::Area, Unit::Acre) => Ok(UomArea::new::<square_meter>(canonical).get::<acre>()),
        (Quantity::Area, Unit::Hectare) => {
            Ok(UomArea::new::<square_meter>(canonical).get::<hectare>())
        }

        (Quantity::Angle, Unit::Radian) => Ok(UomAngle::new::<radian>(canonical).get::<radian>()),
        (Quantity::Angle, Unit::Degree) => {
            Ok(UomAngle::new::<radian>(canonical).get::<angle_degree>())
        }

        (Quantity::Frequency, Unit::Hertz) => {
            Ok(UomFrequency::new::<hertz>(canonical).get::<hertz>())
        }
        (Quantity::Frequency, Unit::Kilohertz) => {
            Ok(UomFrequency::new::<hertz>(canonical).get::<kilohertz>())
        }
        (Quantity::Frequency, Unit::Megahertz) => {
            Ok(UomFrequency::new::<hertz>(canonical).get::<megahertz>())
        }

        (q, u) => Err(UnitError::UnitNotInQuantity {
            quantity: q,
            unit: u,
        }),
    }
}
