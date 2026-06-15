//! Validate a record's content against its collection definition.
//!
//! The pure contract-check the gate's validate step runs
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Per-kind validation in the gate
//! write path"). It checks declared fields only: every `required` field must be
//! present and non-null, and every present field must match its declared type.
//! Undeclared fields are **allowed** — validation narrows the declared shape but
//! does not forbid extra content, so a tenant can adopt a collection
//! incrementally without a migration that strips unmodelled fields. Uniqueness
//! is *not* checked here (it is an index-level concern with its own migration
//! question, `BACKEND-COLLECTIONS.md` open question 11).

use serde_json::Value;

use super::def::CollectionDef;

/// One field that failed validation, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFailure {
    /// The field name that failed.
    pub field: String,
    /// Human-readable reason (missing required, or type mismatch).
    pub reason: String,
}

/// The collected failures from validating content against a collection.
///
/// Empty failures never construct one of these — [`CollectionDef::validate`]
/// returns `Ok(())` when nothing failed, so a [`ValidationError`] always names at
/// least one problem.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("content does not satisfy collection `{collection}`: {}", render(.failures))]
pub struct ValidationError {
    /// The collection name the content was validated against.
    pub collection: String,
    /// The fields that failed, in schema declaration order.
    pub failures: Vec<FieldFailure>,
}

/// Render failures as a compact `field: reason; field: reason` list.
fn render(failures: &[FieldFailure]) -> String {
    failures
        .iter()
        .map(|f| format!("{}: {}", f.field, f.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

impl CollectionDef {
    /// Validate `content` against this collection's declared fields.
    ///
    /// Checks each declared field's presence (when `required`) and type. Returns
    /// every failure at once so a caller (and a UI) sees the full set, not just
    /// the first.
    ///
    /// # Errors
    /// Returns a [`ValidationError`] listing each declared field that is missing
    /// while required, or present with the wrong type.
    pub fn validate(&self, content: &Value) -> Result<(), ValidationError> {
        let mut failures = Vec::new();

        for field in &self.schema {
            match content.get(&field.name) {
                None | Some(Value::Null) => {
                    if field.required {
                        failures.push(FieldFailure {
                            field: field.name.clone(),
                            reason: "required field is missing".to_owned(),
                        });
                    }
                }
                Some(value) => {
                    if !field.field_type.accepts(value) {
                        failures.push(FieldFailure {
                            field: field.name.clone(),
                            reason: format!("expected {}", field.field_type.as_str()),
                        });
                    }
                }
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(ValidationError {
                collection: self.name.clone(),
                failures,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CollectionDef;
    use serde_json::json;

    fn site() -> CollectionDef {
        CollectionDef::parse(&json!({
            "name": "site",
            "schema": [
                { "name": "key",  "type": "text",   "required": true },
                { "name": "name", "type": "text",   "required": true },
                { "name": "area", "type": "number" }
            ]
        }))
        .expect("parse")
    }

    #[test]
    fn a_well_formed_record_validates() {
        let ok = site().validate(&json!({
            "kind": "site", "key": "s1", "name": "HQ", "area": 1200
        }));
        assert!(ok.is_ok());
    }

    #[test]
    fn extra_undeclared_fields_are_allowed() {
        let ok = site().validate(&json!({
            "kind": "site", "key": "s1", "name": "HQ", "notes": "extra"
        }));
        assert!(ok.is_ok());
    }

    #[test]
    fn a_missing_required_field_fails() {
        let err = site()
            .validate(&json!({ "kind": "site", "name": "HQ" }))
            .expect_err("missing key");
        assert_eq!(err.failures.len(), 1);
        assert_eq!(err.failures[0].field, "key");
    }

    #[test]
    fn a_type_mismatch_fails() {
        let err = site()
            .validate(&json!({ "kind": "site", "key": "s1", "name": "HQ", "area": "big" }))
            .expect_err("area not a number");
        assert_eq!(err.failures[0].field, "area");
    }

    #[test]
    fn all_failures_are_reported_at_once() {
        let err = site()
            .validate(&json!({ "kind": "site", "area": false }))
            .expect_err("multiple failures");
        // key missing, name missing, area wrong type.
        assert_eq!(err.failures.len(), 3);
    }

    #[test]
    fn an_optional_absent_field_is_fine() {
        let ok = site().validate(&json!({ "kind": "site", "key": "s1", "name": "HQ" }));
        assert!(ok.is_ok());
    }
}
