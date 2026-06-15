//! A record DTO is rendered per the user's unit and datetime preferences.
//!
//! End-to-end of the Preferences component (`rubix/docs/sessions/WS-16.md`): a
//! stored canonical record (metric, RFC 3339 UTC) is displayed converted for an
//! imperial user and re-formatted for the user's datetime pattern, while fields
//! the preferences do not own pass through untouched.

use rubix_prefs::{DateTimePattern, FieldSpec, Preferences, Quantity, UnitSystem, apply_to};
use serde_json::json;

fn record_fields() -> Vec<FieldSpec<'static>> {
    vec![
        FieldSpec::Measure {
            name: "temperature",
            quantity: Quantity::Temperature,
        },
        FieldSpec::Measure {
            name: "wind",
            quantity: Quantity::Speed,
        },
        FieldSpec::Timestamp { name: "created" },
    ]
}

#[test]
fn imperial_user_sees_converted_values_and_their_datetime_pattern() {
    let prefs = Preferences::new(UnitSystem::Imperial, DateTimePattern::new("%d %b %Y %H:%M"));
    let mut dto = json!({
        "id": "rec-1",
        "temperature": 0.0,
        "wind": 10.0,
        "created": "2026-06-15T09:30:00Z",
        "note": "outside",
    });

    apply_to(&mut dto, &prefs, &record_fields()).expect("apply prefs");

    assert!((dto["temperature"].as_f64().expect("temp") - 32.0).abs() < 1e-9);
    assert!((dto["wind"].as_f64().expect("wind") - 22.369_362_92).abs() < 1e-6);
    assert_eq!(dto["created"], json!("15 Jun 2026 09:30"));
    assert_eq!(dto["note"], json!("outside"));
    assert_eq!(dto["id"], json!("rec-1"));
}

#[test]
fn metric_user_with_default_pattern_is_canonical() {
    let prefs = Preferences::default();
    let mut dto = json!({
        "temperature": 21.5,
        "wind": 3.0,
        "created": "2026-06-15T09:30:00Z",
    });

    apply_to(&mut dto, &prefs, &record_fields()).expect("apply prefs");

    assert!((dto["temperature"].as_f64().expect("temp") - 21.5).abs() < f64::EPSILON);
    assert!((dto["wind"].as_f64().expect("wind") - 3.0).abs() < f64::EPSILON);
    assert_eq!(dto["created"], json!("2026-06-15 09:30:00"));
}
