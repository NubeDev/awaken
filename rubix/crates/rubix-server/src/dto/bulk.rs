//! Wire shapes for bulk record CRUD (`POST /records/bulk`, `BULK-AND-JOBS.md`).
//!
//! An envelope of **keyed items**, each an op over free-form content, with
//! per-item status — the `query/batch` shape extended to mutations. The status of
//! item N is reported under item N's `key`, whether it comes back inline (Tier-1,
//! HTTP 200) or over the WS stream after promotion (Tier-2): the `key` is the
//! correlation that lets the client reassemble the full result from the `202` body
//! plus the frames with no gap and no double-report.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The op a bulk item applies, mirroring the single-record routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BulkOp {
    /// Create a record carrying `body` (a server-minted id is returned).
    Create,
    /// Replace record `key`'s content with `body`.
    Update,
    /// Delete record `key`.
    Delete,
}

/// One keyed item in a bulk envelope.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BulkRecordItem {
    /// The caller's correlation key for this item. For `update`/`delete` it is also
    /// the target record id; for `create` it is an arbitrary client tag echoed back
    /// on the status (the new record's id is minted server-side and returned in
    /// `id`).
    pub key: String,
    /// The op to apply.
    pub op: BulkOp,
    /// The free-form content for `create`/`update`; ignored for `delete`.
    #[serde(default)]
    pub body: Option<Value>,
}

/// How the caller wants the bulk op handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BulkMode {
    /// Server-decided: run synchronously, promoting to a job if the soft deadline
    /// is exceeded (the default — sync-vs-async is a server decision).
    #[default]
    Auto,
    /// Force a background job from the start (the caller knows the work is heavy).
    Async,
}

/// The body of a bulk record request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BulkRecordsRequest {
    /// The keyed items to apply, each gated and audited individually through the
    /// gate's `apply()` — bulk is a server-side fan-out, never a permission shortcut.
    pub items: Vec<BulkRecordItem>,
    /// How to handle the op (server-decided by default).
    #[serde(default)]
    pub mode: BulkMode,
}

/// One item's outcome: the op verb (or `failed`), the stored id, or the error.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkItemStatus {
    /// The caller's key for this item.
    pub key: String,
    /// The outcome verb: `created`/`updated`/`deleted`, or `failed`.
    pub status: String,
    /// The stored record id (for a committed create/update).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The failure message (for a `failed` item).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BulkItemStatus {
    /// A committed item carrying its outcome verb and (optionally) the stored id.
    #[must_use]
    pub fn committed(key: String, status: &str, id: Option<String>) -> Self {
        Self {
            key,
            status: status.to_owned(),
            id,
            error: None,
        }
    }

    /// A failed item carrying its per-item error.
    #[must_use]
    pub fn failed(key: String, error: String) -> Self {
        Self {
            key,
            status: "failed".to_owned(),
            id: None,
            error: Some(error),
        }
    }
}

/// The Tier-1 (synchronous) response: one keyed status per item, HTTP 200.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkRecordsResponse {
    /// One status per input item, matched by `key`.
    pub items: Vec<BulkItemStatus>,
}

/// The Tier-2 (promoted) response: a job handle plus the statuses of every item
/// that committed **before** promotion, HTTP 202. The WS stream carries the rest,
/// keyed by the same `key`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BulkPromotedResponse {
    /// The job's id.
    pub job_id: String,
    /// The opaque ticket to observe the job (returned once).
    pub ticket: String,
    /// When the ticket expires (RFC 3339, UTC).
    pub expires: String,
    /// The statuses of items that committed before promotion (the WS stream carries
    /// the remainder, so the union is the full result).
    pub committed: Vec<BulkItemStatus>,
}
