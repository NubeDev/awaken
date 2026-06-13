//! Wire-shape tests for the preferences DTOs.
//!
//! These tests pin the JSON form byte-for-byte against the SCOPE
//! `preferences_user` column-comment strings. If any rename slips,
//! a downstream OpenAPI client or column migration will silently
//! break — catch it here.

use super::*;
use crate::units::Unit;
use serde_json::json;

// ---------------------------------------------------------------------
// Per-enum wire spellings — column-comment strings byte-for-byte.
// ---------------------------------------------------------------------

#[test]
fn unit_system_wire_spellings() {
    assert_eq!(
        serde_json::to_value(UnitSystem::Metric).unwrap(),
        json!("metric")
    );
    assert_eq!(
        serde_json::to_value(UnitSystem::Imperial).unwrap(),
        json!("imperial")
    );
}

#[test]
fn theme_wire_spellings() {
    assert_eq!(serde_json::to_value(Theme::Light).unwrap(), json!("light"));
    assert_eq!(serde_json::to_value(Theme::Dark).unwrap(), json!("dark"));
    assert_eq!(
        serde_json::to_value(Theme::System).unwrap(),
        json!("system")
    );
}

#[test]
fn date_format_wire_spellings() {
    assert_eq!(
        serde_json::to_value(DateFormat::Auto).unwrap(),
        json!("auto")
    );
    assert_eq!(
        serde_json::to_value(DateFormat::IsoYMD).unwrap(),
        json!("YYYY-MM-DD")
    );
    assert_eq!(
        serde_json::to_value(DateFormat::DmySlash).unwrap(),
        json!("DD/MM/YYYY")
    );
    assert_eq!(
        serde_json::to_value(DateFormat::MdySlash).unwrap(),
        json!("MM/DD/YYYY")
    );
}

#[test]
fn time_format_wire_spellings() {
    assert_eq!(
        serde_json::to_value(TimeFormat::Auto).unwrap(),
        json!("auto")
    );
    assert_eq!(serde_json::to_value(TimeFormat::H24).unwrap(), json!("24h"));
    assert_eq!(serde_json::to_value(TimeFormat::H12).unwrap(), json!("12h"));
}

#[test]
fn week_start_wire_spellings() {
    assert_eq!(
        serde_json::to_value(WeekStart::Auto).unwrap(),
        json!("auto")
    );
    assert_eq!(
        serde_json::to_value(WeekStart::Monday).unwrap(),
        json!("monday")
    );
    assert_eq!(
        serde_json::to_value(WeekStart::Sunday).unwrap(),
        json!("sunday")
    );
}

#[test]
fn number_format_wire_spellings() {
    assert_eq!(
        serde_json::to_value(NumberFormat::Auto).unwrap(),
        json!("auto")
    );
    assert_eq!(
        serde_json::to_value(NumberFormat::CommaDot).unwrap(),
        json!("1,234.56")
    );
    assert_eq!(
        serde_json::to_value(NumberFormat::DotComma).unwrap(),
        json!("1.234,56")
    );
    assert_eq!(
        serde_json::to_value(NumberFormat::SpaceComma).unwrap(),
        json!("1 234,56")
    );
}

#[test]
fn enum_deserialise_round_trip() {
    // Round-trip every enum from its wire form back to its variant.
    let dfs = [
        (json!("auto"), DateFormat::Auto),
        (json!("YYYY-MM-DD"), DateFormat::IsoYMD),
        (json!("DD/MM/YYYY"), DateFormat::DmySlash),
        (json!("MM/DD/YYYY"), DateFormat::MdySlash),
    ];
    for (wire, variant) in dfs {
        assert_eq!(serde_json::from_value::<DateFormat>(wire).unwrap(), variant);
    }
    assert_eq!(
        serde_json::from_value::<NumberFormat>(json!("1 234,56")).unwrap(),
        NumberFormat::SpaceComma,
    );
    assert_eq!(
        serde_json::from_value::<TimeFormat>(json!("12h")).unwrap(),
        TimeFormat::H12,
    );
}

// ---------------------------------------------------------------------
// ResolvedPreferences — full JSON round-trip with every enum variant
// exercised at least once across the test set.
// ---------------------------------------------------------------------

fn sample_resolved() -> ResolvedPreferences {
    ResolvedPreferences {
        timezone: "Europe/Paris".to_owned(),
        locale: "fr-FR".to_owned(),
        language: "fr".to_owned(),
        unit_system: UnitSystem::Metric,
        temperature_unit: Unit::Celsius,
        pressure_unit: Unit::Kilopascal,
        speed_unit: Unit::MeterPerSecond,
        length_unit: Unit::Meter,
        mass_unit: Unit::Kilogram,
        date_format: DateFormat::IsoYMD,
        time_format: TimeFormat::H24,
        week_start: WeekStart::Monday,
        number_format: NumberFormat::SpaceComma,
        currency: "EUR".to_owned(),
        theme: Theme::Dark,
    }
}

#[test]
fn resolved_preferences_json_round_trip() {
    let prefs = sample_resolved();
    let json_value = serde_json::to_value(&prefs).unwrap();
    assert_eq!(
        json_value,
        json!({
            "timezone": "Europe/Paris",
            "locale": "fr-FR",
            "language": "fr",
            "unit_system": "metric",
            "temperature_unit": "celsius",
            "pressure_unit": "kilopascal",
            "speed_unit": "meter_per_second",
            "length_unit": "meter",
            "mass_unit": "kilogram",
            "date_format": "YYYY-MM-DD",
            "time_format": "24h",
            "week_start": "monday",
            "number_format": "1 234,56",
            "currency": "EUR",
            "theme": "dark",
        }),
    );
    let decoded: ResolvedPreferences = serde_json::from_value(json_value).unwrap();
    assert_eq!(decoded, prefs);
}

#[test]
fn resolved_preferences_imperial_variants() {
    // Cover the imperial side of the enum surface.
    let prefs = ResolvedPreferences {
        timezone: "America/New_York".to_owned(),
        locale: "en-US".to_owned(),
        language: "en".to_owned(),
        unit_system: UnitSystem::Imperial,
        temperature_unit: Unit::Fahrenheit,
        pressure_unit: Unit::Psi,
        speed_unit: Unit::MilePerHour,
        length_unit: Unit::Foot,
        mass_unit: Unit::Pound,
        date_format: DateFormat::MdySlash,
        time_format: TimeFormat::H12,
        week_start: WeekStart::Sunday,
        number_format: NumberFormat::CommaDot,
        currency: "USD".to_owned(),
        theme: Theme::Light,
    };
    let decoded: ResolvedPreferences =
        serde_json::from_value(serde_json::to_value(&prefs).unwrap()).unwrap();
    assert_eq!(decoded, prefs);
}

#[test]
fn resolved_preferences_remaining_variants() {
    // Cover the variants the two samples above don't hit:
    // DateFormat::DmySlash, NumberFormat::DotComma, Theme::System,
    // pressure Bar, speed KilometerPerHour, speed Knot.
    let prefs = ResolvedPreferences {
        timezone: "Europe/Berlin".to_owned(),
        locale: "de-DE".to_owned(),
        language: "de".to_owned(),
        unit_system: UnitSystem::Metric,
        temperature_unit: Unit::Celsius,
        pressure_unit: Unit::Bar,
        speed_unit: Unit::KilometerPerHour,
        length_unit: Unit::Meter,
        mass_unit: Unit::Kilogram,
        date_format: DateFormat::DmySlash,
        time_format: TimeFormat::H24,
        week_start: WeekStart::Monday,
        number_format: NumberFormat::DotComma,
        currency: "EUR".to_owned(),
        theme: Theme::System,
    };
    let decoded: ResolvedPreferences =
        serde_json::from_value(serde_json::to_value(&prefs).unwrap()).unwrap();
    assert_eq!(decoded, prefs);

    // And Knot once.
    let prefs = ResolvedPreferences {
        speed_unit: Unit::Knot,
        ..prefs
    };
    let decoded: ResolvedPreferences =
        serde_json::from_value(serde_json::to_value(&prefs).unwrap()).unwrap();
    assert_eq!(decoded.speed_unit, Unit::Knot);
}

// ---------------------------------------------------------------------
// PreferencesPatch — Option semantics on the wire.
// ---------------------------------------------------------------------

#[test]
fn patch_default_is_all_none_and_serialises_to_empty_object() {
    let patch = PreferencesPatch::default();
    let v = serde_json::to_value(&patch).unwrap();
    assert_eq!(v, json!({}));
}

#[test]
fn patch_mixed_some_none_round_trip() {
    let patch = PreferencesPatch {
        temperature_unit: Some(Unit::Fahrenheit),
        date_format: Some(DateFormat::MdySlash),
        theme: Some(Theme::System),
        ..PreferencesPatch::default()
    };
    let v = serde_json::to_value(&patch).unwrap();
    assert_eq!(
        v,
        json!({
            "temperature_unit": "fahrenheit",
            "date_format": "MM/DD/YYYY",
            "theme": "system",
        }),
    );
    let decoded: PreferencesPatch = serde_json::from_value(v).unwrap();
    assert_eq!(decoded, patch);
}

#[test]
fn patch_accepts_missing_fields_as_none() {
    // The route layer disambiguates "missing" vs "explicit null"
    // upstream of this DTO; serde collapses both to None.
    let v = json!({ "unit_system": "imperial" });
    let decoded: PreferencesPatch = serde_json::from_value(v).unwrap();
    assert_eq!(decoded.unit_system, Some(UnitSystem::Imperial));
    assert_eq!(decoded.locale, None);
    assert_eq!(decoded.theme, None);
}

#[test]
fn patch_full_round_trip() {
    // Every field Some — symmetric to ResolvedPreferences shape.
    let patch = PreferencesPatch {
        timezone: Some("Europe/Paris".to_owned()),
        locale: Some("fr-FR".to_owned()),
        language: Some("fr".to_owned()),
        unit_system: Some(UnitSystem::Metric),
        temperature_unit: Some(Unit::Celsius),
        pressure_unit: Some(Unit::Kilopascal),
        speed_unit: Some(Unit::MeterPerSecond),
        length_unit: Some(Unit::Meter),
        mass_unit: Some(Unit::Kilogram),
        date_format: Some(DateFormat::IsoYMD),
        time_format: Some(TimeFormat::H24),
        week_start: Some(WeekStart::Monday),
        number_format: Some(NumberFormat::SpaceComma),
        currency: Some("EUR".to_owned()),
        theme: Some(Theme::Dark),
    };
    let v = serde_json::to_value(&patch).unwrap();
    let decoded: PreferencesPatch = serde_json::from_value(v).unwrap();
    assert_eq!(decoded, patch);
}
