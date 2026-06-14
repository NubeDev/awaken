//! Emit a span onto the WS-07 in-process control bus.
//!
//! `rubix/docs/SCOPE.md`, "Tracing": spans are emitted to the event bus as work
//! flows ingest → pre-process → rule → insight → sink. This verb publishes a span
//! as a [`ControlEvent`] on the in-process plane so live subscribers (a waterfall
//! view, an extension) observe the flow in real time; the durable, bounded copy
//! is a separate concern handled by [`persist`](crate::persist). The event's
//! correlation id is the span's `trace_id`, so a subscriber threads the span back
//! to its originating action exactly as every other bus event (contract #3,
//! `rubix/STACK-DEISGN.md`).

use rubix_bus::{ControlBus, ControlEvent, publish};

use crate::span::Span;

/// The control-event type every emitted span carries.
///
/// Subscribers filter on this single type to receive the span stream; the span
/// detail (name, parent, attributes, timing) rides in the payload.
pub const SPAN_EVENT_TYPE: &str = "trace.span";

/// Publish `span` on `bus`'s in-process control plane, returning the subscriber
/// reach count.
///
/// The span is serialized into the event payload and threaded by its `trace_id`
/// (the WS-05 correlation id). Like every control publish this is fire-and-forget
/// — emitting to a span type with no subscribers is a no-op returning `0`, never
/// a failure, so a traced operation never depends on a listener existing
/// (`rubix/docs/SCOPE.md`, "Event bus").
pub fn emit_span(bus: &ControlBus, span: &Span) -> usize {
    let payload = serde_json::json!({
        "span_id": span.span_id.as_str(),
        "parent_span_id": span.parent_span_id.as_ref().map(rubix_core::Id::as_str),
        "name": span.name,
        "attributes": span.attributes,
        "start_ns": span.start_ns,
        "end_ns": span.end_ns,
    });
    let event = ControlEvent::new(SPAN_EVENT_TYPE, payload, span.trace_id.clone());
    publish(bus, event)
}
