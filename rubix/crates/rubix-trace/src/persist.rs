//! Append a span to the bounded `trace` table — the only durable write path.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`): traces are append-only and
//! bounded/sampled. This verb writes one [`Span`] to the per-namespace `trace`
//! table on the root/owner store handle (the only session permitted past the
//! table's `FOR create … NONE` permission, see [`define`](crate::define)), keyed
//! by the span's own id so appends never collide. Sampling is applied first: a
//! span the configured [`SampleRate`] drops is never written, and the caller is
//! told so via the return value.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Result, TraceError};
use crate::row::{SpanRow, TRACE_TABLE};
use crate::sample::SampleRate;
use crate::span::Span;

/// The outcome of a persist attempt: the span was written, or sampling dropped
/// it before the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persisted {
    /// The span passed sampling and was appended to the trace table.
    Written,
    /// Sampling dropped the span; nothing was written.
    Dropped,
}

/// Append `span` to the `trace` table under `namespace`, subject to `rate`.
///
/// If `rate` drops the span, returns [`Persisted::Dropped`] and writes nothing —
/// the deliberate thinning that keeps high-volume traces bounded (contract #4).
/// Otherwise the span is appended keyed by its own id and [`Persisted::Written`]
/// is returned. Runs on the owner session because the `trace` table denies
/// `create` to every scoped principal.
///
/// # Errors
/// Returns [`TraceError::Persist`] if the append fails.
pub async fn persist_span(
    db: &Surreal<Db>,
    namespace: &str,
    span: &Span,
    rate: SampleRate,
) -> Result<Persisted> {
    if !rate.admits(span) {
        return Ok(Persisted::Dropped);
    }
    let row = SpanRow::from_span(span, namespace);
    let _: Option<SpanRow> = db
        .create((TRACE_TABLE, span.span_id.as_str()))
        .content(row)
        .await
        .map_err(TraceError::Persist)?;
    Ok(Persisted::Written)
}
