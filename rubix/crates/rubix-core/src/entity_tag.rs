//! The entity-tag model (docs/design/page-context-and-nav.md §3): an org-scoped
//! `(org, kind, entity_id, key, value)` store of free-form tags on a domain
//! entity. Tags here are **behaviour-affecting** — a dashboard's tags feed
//! `PageContext.tags` and drive queries — so the write/read handlers enforce the
//! entity's own authz. The value reaches SQL only as a bound parameter (the
//! injection boundary), so a `value` of `'); DROP …` binds as a literal.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::CoreError;

/// The kinds of entity a tag may attach to. Closed: each kind maps to a domain
/// entity whose own authz the tag handlers reuse. Today only `dashboard` is a
/// behaviour-affecting tag surface, but the store is kind-generic so other
/// entities can adopt tags without a schema change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TagEntityKind {
    Dashboard,
}

impl TagEntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TagEntityKind::Dashboard => "dashboard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "dashboard" => Some(TagEntityKind::Dashboard),
            _ => None,
        }
    }
}

/// A full tag set on one entity: `key → value`. A `null` value is a marker tag
/// (presence without a value); a string value is a value tag. Replaced wholesale
/// by the `PUT` route (the editor owns the full set).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct EntityTags(pub BTreeMap<String, Option<String>>);

impl EntityTags {
    /// Keys must be non-empty `[a-zA-Z0-9_]` (same rule as [`crate::TagSet`]), so
    /// a tag key is a stable, addressable token (a `tag`-source variable reads it
    /// by key). Values are free-form and unvalidated — they bind as parameters.
    pub fn validate(&self) -> Result<(), CoreError> {
        for key in self.0.keys() {
            if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(CoreError::InvalidTag(key.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_token_round_trips() {
        assert_eq!(TagEntityKind::parse("dashboard"), Some(TagEntityKind::Dashboard));
        assert!(TagEntityKind::parse("point").is_none());
    }

    #[test]
    fn keys_validated_values_are_free_form() {
        let mut tags = EntityTags::default();
        // An injection-shaped value is accepted — it binds as a parameter, never
        // executes (docs/design/page-context-and-nav.md "Injection boundary").
        tags.0.insert("building".into(), Some("'); DROP TABLE dashboards; --".into()));
        tags.0.insert("marker".into(), None);
        assert!(tags.validate().is_ok());
        // A malformed key is rejected.
        tags.0.insert("bad key".into(), Some("x".into()));
        assert!(tags.validate().is_err());
    }
}
