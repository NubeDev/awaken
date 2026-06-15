//! Tracing for the rubix platform — correlated spans, emitted and bounded.
//!
//! `rubix/docs/SCOPE.md`, "Tracing" (one of the three cross-cutting concerns that
//! fall out of the gate + bus chokepoints, distinct from audit and undo):
//!
//! - A [`Span`] records one step as work flows ingest → pre-process → rule →
//!   insight → sink. Its `trace_id` is the WS-05 [`CorrelationId`] minted at the
//!   gate or at ingest, so every span of one operation shares it (contract #3,
//!   `rubix/STACK-DEISGN.md`). Spans link by `parent_span_id` into a tree.
//! - [`emit_span`] publishes a span onto the WS-07 in-process control bus so live
//!   subscribers observe the flow.
//! - [`persist_span`] appends a span to the bounded, append-only `trace` table,
//!   subject to a [`SampleRate`] (drop fraction from `RUBIX_TRACE_SAMPLE`).
//! - [`enforce_retention`] caps a namespace's stored spans, evicting the oldest —
//!   traces are high volume and not kept forever (contract #4).
//! - [`assemble_trace`] reads spans back and links them into [`SpanNode`] trees
//!   by trace id — the hook WS-11's Rhai per-evaluation span tree plugs into.
//!
//! [`CorrelationId`]: rubix_core::CorrelationId

mod assemble;
mod define;
mod emit;
mod error;
mod persist;
mod retain;
mod row;
mod sample;
mod span;
mod span_metrics;
mod summary;

pub use assemble::{SpanNode, assemble_trace};
pub use define::define_trace_schema;
pub use emit::{SPAN_EVENT_TYPE, emit_span};
pub use error::{Result, TraceError};
pub use persist::{Persisted, persist_span};
pub use retain::enforce_retention;
pub use sample::SampleRate;
pub use span::Span;
pub use span_metrics::{
    MetricsBuilder, SpanMetrics, SpanStatus, SPAN_COST, SPAN_KIND, SPAN_STATUS, SPAN_TOKENS,
    read_kind,
};
pub use summary::{TraceSummary, read_summary, upsert_summary};
