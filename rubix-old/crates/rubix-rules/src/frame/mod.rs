//! The `Frame` handle — the DataFrame the curated primitives operate on.
//!
//! A `Frame` is the Rhai custom type a script chains primitives on
//! (`df.resample(...).zscore("kw").anomalies("kw", 3.0)`). It wraps the Arrow
//! `RecordBatch`es the caller handed in (the crate never queries a database) and
//! a DataFusion-first compute path: every primitive registers the batches as an
//! in-memory table in a fresh `SessionContext`, runs one SQL statement, and
//! collects the result into a new `Frame`.
//!
//! Why SQL rather than hand-built logical plans: DataFusion's SQL surface gives
//! window `RANGE` frames (for time-duration `rolling_*`), `date_bin` (for
//! `resample`), and aggregates directly, and keeps each primitive a few lines.
//! Crucially it makes the **no-row-explosion invariant** auditable — every
//! primitive is a projection / filter / aggregate over the single registered
//! table, never a join, and [`Frame::compute`] asserts the output row count
//! never exceeds the input. The sandbox's size limits cannot catch an explosion
//! inside the engine, so the surface itself forbids it.
//!
//! Rhai is synchronous and DataFusion is async, so compute blocks on a
//! current-thread runtime built per call — cheap, and it keeps one engine /
//! execution with no shared async state across tenants.

mod anomalies;
mod any_true;
mod compute;
mod describe;
mod duration;
mod fill_null;
mod filter;
mod head;
mod lag;
mod rename;
mod resample;
mod rolling;
mod select;
mod sort;
mod zscore;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use datafusion::arrow::datatypes::SchemaRef;
use datafusion::arrow::record_batch::RecordBatch;

/// The in-engine table name each primitive registers its input under.
pub(crate) const TABLE: &str = "f";

/// A handle to an immutable set of rows, chained through curated primitives.
///
/// Cloning is cheap (the batches are `Arc`-shared); each primitive returns a new
/// `Frame` rather than mutating, so a script can branch off intermediate frames.
#[derive(Clone)]
pub struct Frame {
    batches: Arc<Vec<RecordBatch>>,
    schema: SchemaRef,
    /// Stable identity for composition memoization keyed by `(name, frame, params)`.
    id: u64,
}

impl Frame {
    /// Build a frame from caller-supplied batches. Entry from the run path.
    pub fn new(schema: SchemaRef, batches: Vec<RecordBatch>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            batches: Arc::new(batches),
            schema,
            id: NEXT.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// The frame's Arrow schema.
    pub fn schema(&self) -> &SchemaRef {
        &self.schema
    }

    /// The frame's batches.
    pub fn batches(&self) -> &[RecordBatch] {
        &self.batches
    }

    /// Total row count across batches.
    pub fn row_count(&self) -> usize {
        self.batches.iter().map(RecordBatch::num_rows).sum()
    }

    /// The frame's composition-memoization identity.
    pub(crate) fn identity(&self) -> u64 {
        self.id
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame")
            .field("rows", &self.row_count())
            .field("columns", &self.schema.fields().len())
            .finish()
    }
}
