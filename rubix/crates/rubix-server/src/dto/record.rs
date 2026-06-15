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
            created: record.created.to_string(),
            updated: record.updated.to_string(),
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
