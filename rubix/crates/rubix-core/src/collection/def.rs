//! The collection definition — a record's content read as a typed contract.
//!
//! A collection is a `kind: "collection"` record, not a table
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "The collection model — a record,
//! not a table"): defining one is a normal gate write, so SCOPE's "the domain is
//! not baked in" holds. This type is the *parsed* view of such a record's
//! `content`, used by the gate's validate step to enforce the shape records of
//! the collection's `name` must carry. Parsing is fail-closed on the field set:
//! an unknown field `type` makes the whole definition invalid rather than
//! admitting an unconstrained field.

use serde_json::Value;

use super::field::{FieldDef, FieldType};

/// The `kind` value a collection-defining record carries, and the value records
/// of a collection match against (`content.kind == collection.name`).
pub const COLLECTION_KIND: &str = "collection";

/// A parsed collection definition: the kind it names plus its typed fields.
///
/// Access-rule expressions (`listRule`/`writeRule`) and `indexes` from the
/// record are preserved as raw JSON on [`CollectionDef::raw`] rather than modelled
/// here — their evaluation surface is a separate open question
/// (`BACKEND-COLLECTIONS.md`, open questions 4 and 11) and modelling them now
/// would imply an enforcement path that does not exist yet.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionDef {
    /// The kind value records of this collection carry in `content.kind`.
    pub name: String,
    /// The typed fields records of this collection must satisfy.
    pub schema: Vec<FieldDef>,
}

/// Why a record's content could not be read as a collection definition.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CollectionParseError {
    /// The content was not a JSON object.
    #[error("collection definition must be a JSON object")]
    NotAnObject,
    /// The `name` field was missing or not a non-empty string.
    #[error("collection definition requires a non-empty string `name`")]
    MissingName,
    /// The `schema` field was present but not an array.
    #[error("collection `schema` must be an array of field definitions")]
    SchemaNotArray,
    /// A field entry was malformed (missing name, or an unknown type).
    #[error("collection field {index} is invalid: {reason}")]
    BadField {
        /// The position of the offending field in the `schema` array.
        index: usize,
        /// What was wrong with it.
        reason: String,
    },
}

impl CollectionDef {
    /// Read a record's `content` as a collection definition.
    ///
    /// The record is expected to be a `kind: "collection"` record; this parses
    /// the definition fields. A missing `schema` is treated as an empty schema
    /// (a collection that constrains nothing but its `name`), which is what the
    /// bootstrap meta-collection relies on.
    ///
    /// # Errors
    /// Returns a [`CollectionParseError`] if the content is not an object, the
    /// `name` is missing/empty, or any field entry is malformed.
    pub fn parse(content: &Value) -> Result<CollectionDef, CollectionParseError> {
        let obj = content
            .as_object()
            .ok_or(CollectionParseError::NotAnObject)?;

        let name = obj
            .get("name")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or(CollectionParseError::MissingName)?
            .to_owned();

        let schema = match obj.get("schema") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(entries)) => entries
                .iter()
                .enumerate()
                .map(|(index, entry)| parse_field(index, entry))
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(CollectionParseError::SchemaNotArray),
        };

        Ok(CollectionDef { name, schema })
    }
}

/// Parse one `schema[]` entry into a [`FieldDef`].
fn parse_field(index: usize, entry: &Value) -> Result<FieldDef, CollectionParseError> {
    let bad = |reason: &str| CollectionParseError::BadField {
        index,
        reason: reason.to_owned(),
    };

    let obj = entry.as_object().ok_or_else(|| bad("not an object"))?;

    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad("missing `name`"))?
        .to_owned();

    let type_str = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("missing `type`"))?;
    let field_type = FieldType::parse(type_str)
        .ok_or_else(|| bad(&format!("unknown type `{type_str}`")))?;

    let required = obj.get("required").and_then(Value::as_bool).unwrap_or(false);
    let unique = obj.get("unique").and_then(Value::as_bool).unwrap_or(false);

    Ok(FieldDef {
        name,
        field_type,
        required,
        unique,
    })
}

#[cfg(test)]
mod tests {
    use super::{CollectionDef, CollectionParseError, FieldType};
    use serde_json::json;

    #[test]
    fn parses_a_full_definition() {
        let def = CollectionDef::parse(&json!({
            "kind": "collection",
            "name": "site",
            "schema": [
                { "name": "key",  "type": "text",   "required": true, "unique": true },
                { "name": "name", "type": "text",   "required": true },
                { "name": "area", "type": "number" }
            ]
        }))
        .expect("parse");

        assert_eq!(def.name, "site");
        assert_eq!(def.schema.len(), 3);
        assert_eq!(def.schema[0].field_type, FieldType::Text);
        assert!(def.schema[0].required);
        assert!(def.schema[0].unique);
        assert!(!def.schema[1].unique);
        assert_eq!(def.schema[2].field_type, FieldType::Number);
        assert!(!def.schema[2].required);
    }

    #[test]
    fn a_missing_schema_is_an_empty_schema() {
        let def = CollectionDef::parse(&json!({ "name": "freeform" })).expect("parse");
        assert!(def.schema.is_empty());
    }

    #[test]
    fn a_missing_name_is_rejected() {
        assert_eq!(
            CollectionDef::parse(&json!({ "schema": [] })),
            Err(CollectionParseError::MissingName)
        );
        assert_eq!(
            CollectionDef::parse(&json!({ "name": "" })),
            Err(CollectionParseError::MissingName)
        );
    }

    #[test]
    fn an_unknown_field_type_fails_closed() {
        let err = CollectionDef::parse(&json!({
            "name": "x",
            "schema": [{ "name": "loc", "type": "geo" }]
        }))
        .expect_err("must reject unknown type");
        assert!(matches!(err, CollectionParseError::BadField { index: 0, .. }));
    }

    #[test]
    fn non_object_content_is_rejected() {
        assert_eq!(
            CollectionDef::parse(&json!("nope")),
            Err(CollectionParseError::NotAnObject)
        );
    }
}
