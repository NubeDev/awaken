//! The persisted shape of a [`Record`] at the SurrealDB boundary.
//!
//! SurrealDB owns the reserved `id` field as a `RecordId`; the domain
//! [`Record`] carries a plain string [`Id`]. This row type maps between the two
//! so the domain stays free of store-specific identity concerns while the write
//! still keys on the record's own id (`rubix/docs/SCOPE.md`, edge-mintable ids).

use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue, ToSql};

use crate::id::Id;

use super::{RECORD_TABLE, Record};

/// SurrealDB-facing record: the reserved `id` thing plus the record fields.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct RecordRow {
    pub(crate) id: RecordId,
    pub(crate) namespace: String,
    pub(crate) content: serde_json::Value,
    pub(crate) created: Datetime,
    pub(crate) updated: Datetime,
}

impl RecordRow {
    /// Project a domain [`Record`] into its persisted row.
    pub(crate) fn from_record(record: &Record) -> Self {
        Self {
            id: RecordId::new(RECORD_TABLE, record.id.as_str()),
            namespace: record.namespace.clone(),
            content: record.content.clone(),
            created: record.created,
            updated: record.updated,
        }
    }

    /// Reconstruct the domain [`Record`] from a persisted row.
    pub(crate) fn into_record(self) -> Record {
        Record {
            id: Id::from_raw(record_key(&self.id)),
            namespace: self.namespace,
            content: self.content,
            created: self.created,
            updated: self.updated,
        }
    }
}

/// The raw string form of a record id's key (the part after `table:`).
///
/// Record keys are minted from string [`Id`]s, so the key is always a string;
/// other key shapes are coerced through their SurrealQL form for completeness.
fn record_key(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        other => other.to_sql(),
    }
}
