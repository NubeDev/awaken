//! The typed fields a collection declares.
//!
//! A collection's schema is a list of [`FieldDef`]s, each naming one field of a
//! record's content and the [`FieldType`] it must carry. The set is a small
//! closed enum (`rubix/docs/design/BACKEND-COLLECTIONS.md`, open question 3):
//! domain shape comes from data (the collection record), never from a baked-in
//! Rust type, so adding a field type is a deliberate enum change here, not an
//! open string. The type names are the stable wire/storage strings a collection
//! record carries in its `schema[].type`.

use serde_json::Value;

/// The type a collection field's value must satisfy.
///
/// Closed by construction: an unrecognised `type` string in a collection record
/// fails to parse rather than being admitted as an unconstrained field, so a
/// typo cannot silently widen the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// A UTF-8 string value.
    Text,
    /// A JSON number (integer or float).
    Number,
    /// A JSON boolean.
    Bool,
    /// A date/time carried as an RFC 3339 string.
    Date,
    /// A file reference object (`{ id, filename, size, contentType }`). The blob
    /// subsystem that produces the bytes is deferred
    /// (`BACKEND-COLLECTIONS.md`, build-order step 6); the field type and its
    /// reference shape are defined here so collections can declare it today.
    File,
    /// A link to another record, carried as that record's id string.
    Relation,
}

impl FieldType {
    /// The stable wire/storage string for this field type.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Number => "number",
            FieldType::Bool => "bool",
            FieldType::Date => "date",
            FieldType::File => "file",
            FieldType::Relation => "relation",
        }
    }

    /// Resolve a collection record's `type` string to a known field type.
    ///
    /// Returns `None` for any unrecognised string so the caller fails closed
    /// (a collection that declares an unknown type is rejected, not admitted
    /// with an unconstrained field).
    #[must_use]
    pub fn parse(raw: &str) -> Option<FieldType> {
        match raw {
            "text" => Some(FieldType::Text),
            "number" => Some(FieldType::Number),
            "bool" => Some(FieldType::Bool),
            "date" => Some(FieldType::Date),
            "file" => Some(FieldType::File),
            "relation" => Some(FieldType::Relation),
            _ => None,
        }
    }

    /// Whether `value` satisfies this field type.
    ///
    /// `null` is never accepted here — presence/absence (the `required` rule) is
    /// decided by the caller before the type check, so a value reaching this
    /// method is a concrete value that must match the declared type.
    #[must_use]
    pub fn accepts(self, value: &Value) -> bool {
        match self {
            FieldType::Text => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Bool => value.is_boolean(),
            // A date is transported as an RFC 3339 string; rubix-core does not
            // pull a date library to parse it (no chrono dependency), so the
            // contract checked here is "string", with format validation left to
            // the boundary that renders it (WS-16 prefs).
            FieldType::Date => value.is_string(),
            // A file field stores a reference object, never raw bytes; the
            // minimal contract is "an object carrying a string id".
            FieldType::File => value
                .as_object()
                .is_some_and(|obj| obj.get("id").is_some_and(Value::is_string)),
            // A relation is the linked record's id string.
            FieldType::Relation => value.is_string(),
        }
    }
}

/// One typed field a collection declares over record content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    /// The field's name within a record's `content` object.
    pub name: String,
    /// The type the field's value must satisfy.
    pub field_type: FieldType,
    /// Whether the field must be present and non-null on every record.
    pub required: bool,
    /// Whether values must be unique across the collection.
    ///
    /// Uniqueness is realised by a SurrealDB `DEFINE INDEX` whose migration is an
    /// open question (`BACKEND-COLLECTIONS.md`, open question 11: adding a unique
    /// index over already-conflicting rows can break a running store). The flag
    /// is parsed and preserved here; index emission is intentionally not done in
    /// the content-validation path so validation never half-applies a schema
    /// migration.
    pub unique: bool,
}

#[cfg(test)]
mod tests {
    use super::{FieldType, Value};
    use serde_json::json;

    #[test]
    fn every_field_type_round_trips_through_its_string() {
        for ft in [
            FieldType::Text,
            FieldType::Number,
            FieldType::Bool,
            FieldType::Date,
            FieldType::File,
            FieldType::Relation,
        ] {
            assert_eq!(FieldType::parse(ft.as_str()), Some(ft));
        }
    }

    #[test]
    fn an_unknown_type_string_resolves_to_none() {
        assert_eq!(FieldType::parse("geo"), None);
        assert_eq!(FieldType::parse(""), None);
    }

    #[test]
    fn accepts_matches_the_declared_type() {
        assert!(FieldType::Text.accepts(&json!("hi")));
        assert!(!FieldType::Text.accepts(&json!(3)));
        assert!(FieldType::Number.accepts(&json!(3.5)));
        assert!(FieldType::Bool.accepts(&json!(true)));
        assert!(FieldType::Date.accepts(&json!("2026-06-15T00:00:00Z")));
        assert!(FieldType::Relation.accepts(&json!("rec-1")));
    }

    #[test]
    fn a_file_field_requires_an_object_with_a_string_id() {
        assert!(FieldType::File.accepts(&json!({ "id": "f-1", "filename": "a.pdf" })));
        assert!(!FieldType::File.accepts(&json!({ "filename": "a.pdf" })));
        assert!(!FieldType::File.accepts(&json!("f-1")));
    }

    #[test]
    fn null_is_never_accepted_as_a_concrete_value() {
        assert!(!FieldType::Text.accepts(&Value::Null));
        assert!(!FieldType::Number.accepts(&Value::Null));
    }
}
