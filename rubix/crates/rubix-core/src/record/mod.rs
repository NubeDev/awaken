//! The generic document record.
//!
//! The platform has no fixed domain ontology — a record is an id, the namespace
//! it belongs to, free-form JSON content, and create/update timestamps
//! (`rubix/docs/SCOPE.md`, principle 4: generic, not domain-specific). Structure
//! comes from tagging on the graph (see [`crate::tag`]), never from a baked-in
//! schema. CRUD verbs execute SurrealQL over a SurrealDB connection borrowed from
//! the `rubix-store` durable handle.

mod create;
mod delete;
mod read;
mod row;
mod update;

pub use create::create_record;
pub use delete::delete_record;
pub use read::read_record;
pub use update::update_record;

pub(crate) use row::RecordRow;

use surrealdb::types::Datetime;

use crate::id::Id;

/// The SurrealDB table every generic record lives in.
pub(crate) const RECORD_TABLE: &str = "record";

/// A schemaless document record.
///
/// `content` is free-form JSON so no domain ontology is baked in. `created` is
/// stamped once at creation; `updated` is bumped on every content update so the
/// edge-partitioned, append-only sync plane can order writes
/// (`rubix/docs/SCOPE.md`, "Append-only data, edge-partitioned").
///
/// The id is the SurrealDB row key; it is mapped to/from the reserved `id`
/// field at the store boundary (see [`row`]) so the domain type stays a plain
/// string id.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// Stable identifier, unique without coordination across edges.
    pub id: Id,
    /// The namespace (tenant) this record belongs to.
    pub namespace: String,
    /// Free-form document content — the platform imposes no shape on it.
    pub content: serde_json::Value,
    /// When the record was first created (UTC).
    pub created: Datetime,
    /// When the record's content was last updated (UTC).
    pub updated: Datetime,
}

impl Record {
    /// Build a new record stamping `created` and `updated` to the same instant.
    ///
    /// The id is freshly minted; callers persist with [`create_record`].
    #[must_use]
    pub fn new(namespace: impl Into<String>, content: serde_json::Value) -> Self {
        let now = Datetime::now();
        Self {
            id: Id::new(),
            namespace: namespace.into(),
            content,
            created: now,
            updated: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RecordRow, Record};

    #[test]
    fn new_stamps_created_and_updated_together() {
        let record = Record::new("rubix", serde_json::json!({ "temp": 21.5 }));
        assert_eq!(record.created, record.updated);
        assert_eq!(record.namespace, "rubix");
    }

    #[test]
    fn record_round_trips_through_the_persisted_row() {
        let record = Record::new("rubix", serde_json::json!({ "k": "v" }));
        let row = RecordRow::from_record(&record);
        let decoded = row.into_record();
        assert_eq!(decoded, record);
    }
}
