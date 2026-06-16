//! Wire shapes for the reading (time-series data plane) resource.
//!
//! Readings are the data plane, not the config plane: append-only, never undone,
//! queried in bulk by window (`rubix/docs/design/READINGS-TIMESERIES.md`). The
//! append request is deliberately lean — the `series` is named **once** for the
//! whole batch and each sample carries only `{ at, value }` (+ optional extras),
//! because the per-sample display metadata (`unit`, `quantity`) lives on the
//! series record, not on every reading. The read DTO mirrors that: `at` (the
//! measurement instant the chart keys its x-axis on) and `value`, plus the bare
//! `series` id so a board joins it to a register by `series === register.id`.

use rubix_core::Reading;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// The body of a bulk-append request: one series, many samples.
///
/// `series` is the series-defining record id, named once for the batch (not
/// repeated per sample). The principal's namespace is the edge partition the
/// readings land in — it is **never** taken from the body, so a publisher cannot
/// write into another edge's partition.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AppendReadingsRequest {
    /// The series-defining record id every sample in this batch belongs to.
    pub series: String,
    /// The samples to append, each a `{ at, value }` (+ optional `content`).
    pub samples: Vec<ReadingSampleDto>,
}

/// One sample in an append batch: a measurement instant and its numeric value.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReadingSampleDto {
    /// The measurement instant (RFC 3339, UTC) — when the world produced the value.
    pub at: String,
    /// The numeric sample.
    pub value: f64,
    /// Free-form extras (quality flags, source key); absent for the common sample.
    #[serde(default)]
    pub content: Option<Value>,
}

/// The result of a bulk append: how many rows the statement wrote.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AppendReadingsResponse {
    /// The number of readings appended (re-appended duplicates are idempotent).
    pub appended: u64,
}

/// A reading as returned to a client by the windowed historian read.
///
/// Lean by design: the x-axis instant `at`, the numeric `value`, and the bare
/// `series` id for the register join. `at` is rendered RFC 3339 so the prefs
/// layer can re-format it per the user's datetime pattern at the boundary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReadingDto {
    /// The bare id of the series-defining record this sample belongs to.
    pub series: String,
    /// The measurement instant (RFC 3339, UTC) — the chart's x-axis key.
    pub at: String,
    /// The numeric sample.
    pub value: f64,
}

impl From<Reading> for ReadingDto {
    fn from(reading: Reading) -> Self {
        Self {
            series: reading.series,
            at: reading.at.to_string(),
            value: reading.value,
        }
    }
}
