//! Apply a user's preferences to a response DTO.
//!
//! The integration point the transport layer (WS-16) calls before a value goes
//! to the wire (`rubix/docs/SCOPE.md`, "Preferences"): rewrite the DTO's tagged
//! fields — convert each numeric field to the user's unit system and re-render
//! each timestamp field in the user's datetime pattern — and leave every other
//! field untouched. The set of fields a DTO owns is declared as a [`FieldSpec`]
//! list, so this verb never guesses which fields carry physical quantities.

use serde_json::Value;

use crate::datetime::format_json;
use crate::error::Result;
use crate::preferences::Preferences;
use crate::units::{Quantity, convert_json};

/// How one DTO field should be rendered for display.
///
/// A field is either a physical [`Quantity`] (convert to the user's unit system)
/// or a timestamp (re-render in the user's datetime pattern). A DTO declares the
/// fields it owns as a slice of these, naming each by its JSON key.
#[derive(Debug, Clone, Copy)]
pub enum FieldSpec<'a> {
    /// A numeric field carrying a physical quantity, keyed by `name`.
    Measure {
        /// The JSON object key of the field.
        name: &'a str,
        /// The physical quantity the stored (metric) value measures.
        quantity: Quantity,
    },
    /// A timestamp field (an RFC 3339 string), keyed by `name`.
    Timestamp {
        /// The JSON object key of the field.
        name: &'a str,
    },
}

impl FieldSpec<'_> {
    /// The JSON key this spec addresses.
    fn name(&self) -> &str {
        match self {
            FieldSpec::Measure { name, .. } | FieldSpec::Timestamp { name } => name,
        }
    }
}

/// Apply `prefs` to the `dto` object, rewriting only the declared `fields`.
///
/// Each field present in `dto` is transformed per its [`FieldSpec`]; fields not
/// present are skipped, and fields not declared are left exactly as they are. The
/// rewrite is in place on `dto`.
///
/// # Errors
/// Returns a [`PrefsError`](crate::PrefsError) if a declared field is present but
/// holds a value the transform cannot interpret (a non-numeric measure or a
/// non-RFC-3339 timestamp) — fail closed rather than emit a mislabelled value.
pub fn apply_to(dto: &mut Value, prefs: &Preferences, fields: &[FieldSpec<'_>]) -> Result<()> {
    let Some(object) = dto.as_object_mut() else {
        return Ok(());
    };
    for spec in fields {
        let Some(current) = object.get(spec.name()) else {
            continue;
        };
        let rendered = match spec {
            FieldSpec::Measure { quantity, .. } => convert_json(current, *quantity, prefs.units)?,
            FieldSpec::Timestamp { .. } => format_json(current, &prefs.datetime)?,
        };
        object.insert(spec.name().to_owned(), rendered);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::datetime::DateTimePattern;
    use crate::units::{Quantity, UnitSystem};

    use super::{FieldSpec, Preferences, apply_to};

    fn fields() -> Vec<FieldSpec<'static>> {
        vec![
            FieldSpec::Measure {
                name: "temp",
                quantity: Quantity::Temperature,
            },
            FieldSpec::Timestamp { name: "at" },
        ]
    }

    #[test]
    fn rewrites_owned_fields_and_leaves_others() {
        let prefs = Preferences::new(UnitSystem::Imperial, DateTimePattern::new("%d/%m/%Y"));
        let mut dto = json!({
            "temp": 100.0,
            "at": "2026-06-15T09:30:00Z",
            "label": "boiler",
        });
        apply_to(&mut dto, &prefs, &fields()).expect("apply");
        assert!((dto["temp"].as_f64().expect("temp") - 212.0).abs() < 1e-9);
        assert_eq!(dto["at"], json!("15/06/2026"));
        assert_eq!(dto["label"], json!("boiler"));
    }

    #[test]
    fn metric_default_leaves_values_unchanged() {
        let prefs = Preferences::default();
        let mut dto = json!({ "temp": 21.5, "at": "2026-06-15T09:30:00Z" });
        apply_to(&mut dto, &prefs, &fields()).expect("apply");
        assert!((dto["temp"].as_f64().expect("temp") - 21.5).abs() < f64::EPSILON);
        assert_eq!(dto["at"], json!("2026-06-15 09:30:00"));
    }

    #[test]
    fn missing_field_is_skipped() {
        let prefs = Preferences::default();
        let mut dto = json!({ "label": "x" });
        apply_to(&mut dto, &prefs, &fields()).expect("apply");
        assert_eq!(dto, json!({ "label": "x" }));
    }

    #[test]
    fn malformed_field_fails_closed() {
        let prefs = Preferences::new(UnitSystem::Imperial, DateTimePattern::default());
        let mut dto = json!({ "temp": "warm" });
        assert!(apply_to(&mut dto, &prefs, &fields()).is_err());
    }

    #[test]
    fn non_object_dto_is_a_noop() {
        let prefs = Preferences::default();
        let mut dto = json!([1, 2, 3]);
        apply_to(&mut dto, &prefs, &fields()).expect("apply");
        assert_eq!(dto, json!([1, 2, 3]));
    }
}
