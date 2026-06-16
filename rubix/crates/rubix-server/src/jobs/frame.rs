//! The typed frames a job streams over the WS job channel.
//!
//! A job's progress and results ride the existing WS plane as JSON text frames
//! (`rubix/docs/design/BULK-AND-JOBS.md`, "Tier 2"): `{ item_key, status }` events
//! for a bulk mutation (the "status per new item"), and result `RecordBatch`
//! chunks for a streamed query, bracketed by a terminal marker. One closed enum
//! so producer (the job task) and consumer (the WS bridge / reconnect replay)
//! agree on the wire without a second schema.

use serde::Serialize;
use serde_json::Value;

use crate::dto::query::ColumnDto;

/// One frame on a job's broadcast/backlog stream.
///
/// `Item` and `Chunk` are progress/result frames; `Done` and `Failed` are the two
/// terminal markers that close the stream. The frame is serialised to a JSON text
/// frame on the wire, tagged by `type`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobFrame {
    /// Per-item status for a bulk mutation, keyed by the submitted item `key` so
    /// the union of the `202` body and these frames is the full result with no gap
    /// and no double-report.
    Item {
        /// The caller's correlation key for this item.
        key: String,
        /// The item's outcome verb (`created`/`updated`/`deleted`/`failed`).
        status: String,
        /// The stored record id, when the item committed a create/update.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// The failure message, when the item failed its per-item authorization or
        /// validation.
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// A chunk of query result rows (Arrow-JSON), streamed for a Tier-2 query job.
    /// `columns` rides the first chunk so a client gets types without sniffing.
    Chunk {
        /// The rows in this chunk, each a JSON object keyed by column name.
        rows: Vec<Value>,
        /// The result columns (name + coarse type); present on the first chunk.
        #[serde(skip_serializing_if = "Option::is_none")]
        columns: Option<Vec<ColumnDto>>,
    },
    /// Terminal success marker — the stream is complete.
    Done,
    /// Terminal failure marker carrying the reason the job ended early.
    Failed {
        /// Why the job failed (`timeout`, `cancelled`, or a producer error).
        reason: String,
    },
}

impl JobFrame {
    /// Whether this frame closes the stream (a `Done` or `Failed`).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobFrame::Done | JobFrame::Failed { .. })
    }
}
