//! Wire shapes for the record resource.
//!
//! The transport DTO for a generic record (`rubix/docs/sessions/WS-16.md`): the
//! id, namespace, free-form content, and timestamps a client sees. Timestamps are
//! rendered as RFC 3339 strings so the WS-16 prefs layer can re-format them per
//! the user's datetime pattern at the boundary. Create/update carry only the
//! free-form content; the platform bakes in no fixed shape (`rubix/docs/SCOPE.md`,
//! principle 4).

use rubix_core::Record;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// A record as returned to a client.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecordDto {
    /// The record's stable id.
    pub id: String,
    /// The namespace (tenant) the record belongs to.
    pub namespace: String,
    /// Free-form document content.
    pub content: Value,
    /// The names of the tags this record carries (`record→tagged→tag`).
    ///
    /// Tags are graph edges, not content — they are projected onto the listing
    /// read so a client can see a record's classification without a second call.
    /// Single-record reads (get/create/update) return an empty set; only the list
    /// read joins the tag graph.
    #[serde(default)]
    pub tags: Vec<String>,
    /// When the record was created (RFC 3339, UTC).
    pub created: String,
    /// When the record's content was last updated (RFC 3339, UTC).
    pub updated: String,
}

impl From<Record> for RecordDto {
    fn from(record: Record) -> Self {
        Self {
            id: record.id.to_string(),
            namespace: record.namespace,
            content: record.content,
            tags: Vec::new(),
            created: record.created.to_string(),
            updated: record.updated.to_string(),
        }
    }
}

impl RecordDto {
    /// Project a record into its DTO with its tag names attached.
    ///
    /// Used by the list read, which joins the tag-graph projection
    /// (`rubix_gate::read_record_tags_on_session`) onto each record.
    #[must_use]
    pub fn with_tags(record: Record, tags: Vec<String>) -> Self {
        Self {
            tags,
            ..Self::from(record)
        }
    }
}

/// The body of a create-record request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateRecordRequest {
    /// The free-form content the new record carries.
    pub content: Value,
}

/// The body of an update-record request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateRecordRequest {
    /// The free-form content that replaces the record's current content.
    pub content: Value,
}
