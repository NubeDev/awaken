//! Resolver unit tests — cover the three SCOPE Smoke-tests-block
//! cases verbatim. See `DOCS/user/scope/SCOPE.md` "Smoke tests".

use super::*;
use crate::preferences::{
    DateFormat, NumberFormat, Theme, TimeFormat, UnitSystem, WeekStart,
};
use crate::units::Unit;

/// Smoke test "Resolver layer precedence" — user wins per-column,
/// org wins where user is NULL, default wins when org is removed.
#[test]
fn resolver_layer_precedence() {
    let defaults = SystemDefaults::starter();

    // -- user wins per column: user sets locale=fr-FR, org sets
    // -- locale=en-AU; user wins.
    let user = UserPrefsRow {
        locale: Some("fr-FR".to_owned()),
        temperature_unit: Some(UnitPref::Explicit(Unit::Fahrenheit)),
        theme: Some(Theme::Dark),
        ..Default::default()
    };
    let org = OrgPrefsRow {
        locale: Some("en-AU".to_owned()),
        language: Some("en".to_owned()),
        temperature_unit: Some(UnitPref::Explicit(Unit::Celsius)),
        unit_system: Some(UnitSystem::Metric),
        ..Default::default()
    };
    let resolved = resolve(Some(user), Some(org), &defaults);
    assert_eq!(resolved.locale, "fr-FR", "user locale wins");
    assert_eq!(
        resolved.temperature_unit,
        Unit::Fahrenheit,
        "user temperature_unit wins"
    );
    // -- org wins for fields the user did not set:
    assert_eq!(
        resolved.language, "en",
        "org language fills NULL user field"
    );
    assert_eq!(
        resolved.unit_system,
        UnitSystem::Metric,
        "org unit_system fills NULL user field"
    );
    // -- theme is user-only:
    assert_eq!(resolved.theme, Theme::Dark);

    // -- default wins when org is removed and user is NULL for the
    // -- field.
    let user_sparse = UserPrefsRow {
        theme: Some(Theme::Dark),
        ..Default::default()
    };
    let resolved = resolve(Some(user_sparse), None, &defaults);
    assert_eq!(
        resolved.locale, "en-US",
        "default locale wins when both layers are NULL"
    );
    assert_eq!(resolved.timezone, "UTC");
    assert_eq!(resolved.temperature_unit, Unit::Celsius);
    assert_eq!(resolved.unit_system, UnitSystem::Metric);
    assert_eq!(resolved.currency, "USD");
    assert_eq!(resolved.date_format, DateFormat::IsoYMD);
    assert_eq!(resolved.time_format, TimeFormat::H24);
    assert_eq!(resolved.week_start, WeekStart::Monday);
    assert_eq!(resolved.number_format, NumberFormat::CommaDot);
    assert_eq!(resolved.theme, Theme::Dark);
}

/// Smoke test "auto derivation" — en-AU + metric + auto-everywhere
/// yields `C / kPa / km/h / m / kg / AUD`.
#[test]
fn auto_derivation_metric_en_au() {
    let user = UserPrefsRow {
        locale: Some("en-AU".to_owned()),
        unit_system: Some(UnitSystem::Metric),
        temperature_unit: Some(UnitPref::Auto),
        pressure_unit: Some(UnitPref::Auto),
        speed_unit: Some(UnitPref::Auto),
        length_unit: Some(UnitPref::Auto),
        mass_unit: Some(UnitPref::Auto),
        currency: Some(StringPref::Auto),
        ..Default::default()
    };
    let resolved = resolve(Some(user), None, &SystemDefaults::starter());
    assert_eq!(resolved.locale, "en-AU");
    assert_eq!(resolved.temperature_unit, Unit::Celsius);
    assert_eq!(resolved.pressure_unit, Unit::Kilopascal);
    assert_eq!(resolved.speed_unit, Unit::KilometerPerHour);
    assert_eq!(resolved.length_unit, Unit::Meter);
    assert_eq!(resolved.mass_unit, Unit::Kilogram);
    assert_eq!(resolved.currency, "AUD");
}

/// Smoke test "auto derivation" (imperial flip) — same row but
/// `unit_system: imperial` yields `F / psi / mph / ft / lb`.
#[test]
fn auto_derivation_imperial_flip() {
    let user = UserPrefsRow {
        locale: Some("en-US".to_owned()),
        unit_system: Some(UnitSystem::Imperial),
        temperature_unit: Some(UnitPref::Auto),
        pressure_unit: Some(UnitPref::Auto),
        speed_unit: Some(UnitPref::Auto),
        length_unit: Some(UnitPref::Auto),
        mass_unit: Some(UnitPref::Auto),
        ..Default::default()
    };
    let resolved = resolve(Some(user), None, &SystemDefaults::starter());
    assert_eq!(resolved.temperature_unit, Unit::Fahrenheit);
    assert_eq!(resolved.pressure_unit, Unit::Psi);
    assert_eq!(resolved.speed_unit, Unit::MilePerHour);
    assert_eq!(resolved.length_unit, Unit::Foot);
    assert_eq!(resolved.mass_unit, Unit::Pound);
}

/// Smoke test "auto derivation" (BBQ case) — org sets metric system,
/// user overrides `temperature_unit: F`. Per the SCOPE BBQ paragraph
/// in R3, the per-unit override wins because it's the more specific
/// layer for that one column; everything else stays metric.
#[test]
fn auto_derivation_bbq_case() {
    let user = UserPrefsRow {
        temperature_unit: Some(UnitPref::Explicit(Unit::Fahrenheit)),
        ..Default::default()
    };
    let org = OrgPrefsRow {
        locale: Some("en-AU".to_owned()),
        unit_system: Some(UnitSystem::Metric),
        temperature_unit: Some(UnitPref::Auto),
        pressure_unit: Some(UnitPref::Auto),
        speed_unit: Some(UnitPref::Auto),
        length_unit: Some(UnitPref::Auto),
        mass_unit: Some(UnitPref::Auto),
        ..Default::default()
    };
    let resolved = resolve(Some(user), Some(org), &SystemDefaults::starter());
    assert_eq!(
        resolved.temperature_unit,
        Unit::Fahrenheit,
        "user override wins for temperature"
    );
    assert_eq!(resolved.pressure_unit, Unit::Kilopascal);
    assert_eq!(resolved.speed_unit, Unit::KilometerPerHour);
    assert_eq!(resolved.length_unit, Unit::Meter);
    assert_eq!(resolved.mass_unit, Unit::Kilogram);
}

#[test]
fn string_pref_parse() {
    assert_eq!(StringPref::parse("auto"), StringPref::Auto);
    assert_eq!(
        StringPref::parse("AUD"),
        StringPref::Explicit("AUD".to_owned())
    );
}

#[test]
fn locale_to_currency_known_regions() {
    assert_eq!(super::locale_to_currency("en-AU").as_deref(), Some("AUD"));
    assert_eq!(super::locale_to_currency("en-GB").as_deref(), Some("GBP"));
    assert_eq!(super::locale_to_currency("en-US").as_deref(), Some("USD"));
    // Language-only tag has no region.
    assert_eq!(super::locale_to_currency("en"), None);
}

#[test]
fn empty_inputs_collapse_to_defaults() {
    let r = resolve(None, None, &SystemDefaults::starter());
    assert_eq!(r.locale, "en-US");
    assert_eq!(r.timezone, "UTC");
    assert_eq!(r.language, "en");
    assert_eq!(r.unit_system, UnitSystem::Metric);
    assert_eq!(r.temperature_unit, Unit::Celsius);
    assert_eq!(r.currency, "USD");
    assert_eq!(r.theme, Theme::System);
}
