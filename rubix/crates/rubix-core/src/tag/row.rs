//! The persisted shape of a [`Tag`] at the SurrealDB boundary.
//!
//! Mirrors `record::row`: SurrealDB owns the reserved `id` thing while the
//! domain [`Tag`] carries a plain string [`Id`].

use surrealdb::types::{RecordId, RecordIdKey, SurrealValue, ToSql};

use crate::id::Id;

use super::{TAG_TABLE, Tag};

/// SurrealDB-facing tag: the reserved `id` thing plus the tag name.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct TagRow {
    pub(crate) id: RecordId,
    pub(crate) name: String,
}

impl TagRow {
    /// Project a domain [`Tag`] into its persisted row.
    pub(crate) fn from_tag(tag: &Tag) -> Self {
        Self {
            id: RecordId::new(TAG_TABLE, tag.id.as_str()),
            name: tag.name.clone(),
        }
    }

    /// Reconstruct the domain [`Tag`] from a persisted row.
    pub(crate) fn into_tag(self) -> Tag {
        Tag {
            id: Id::from_raw(tag_key(&self.id)),
            name: self.name,
        }
    }
}

/// The raw string form of a tag id's key (the part after `tag:`).
fn tag_key(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => other.to_sql(),
    }
}
