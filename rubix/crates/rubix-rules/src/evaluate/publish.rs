//! Publish a rule's recorded insight as a data-change event on the WS-07 bus.
//!
//! A rule's decision is recorded back to SurrealDB **and published as a
//! data-change event** (`rubix/docs/SCOPE.md`, "Rhai — rules and insights"). The
//! durable record is written through the gate ([`record`](super::record)); this
//! verb announces it on the in-process control plane so live components (a
//! dashboard feed, an extension, a sink) observe the firing without polling. The
//! event carries the same correlation id the insight and span tree carry
//! (contract #3, `rubix/STACK-DEISGN.md`), so a subscriber can pivot from the
//! firing to its audit row and its evaluation trace.

use rubix_bus::{ControlBus, ControlEvent, publish};
use rubix_core::{CorrelationId, Id};

use crate::engine::Decision;

/// The control-event type every recorded-insight firing is published under.
///
/// Subscribers filter on this single type to receive insight firings; the
/// insight id, output kind, and decision ride in the payload.
pub const INSIGHT_EVENT_TYPE: &str = "insight.recorded";

/// Publish the firing of `insight_id` (`output` kind) carrying `decision`,
/// threaded by `correlation`, on `bus`.
///
/// Returns the subscriber reach count. Like every control publish this is
/// fire-and-forget — a firing with no subscribers reaches zero and is not an
/// error, so recording an insight never depends on a listener existing
/// (`rubix/docs/SCOPE.md`, "Event bus").
pub fn publish_insight(
    bus: &ControlBus,
    insight_id: &Id,
    output: &str,
    decision: &Decision,
    correlation: &CorrelationId,
) -> usize {
    let payload = serde_json::json!({
        "insight_id": insight_id.as_str(),
        "kind": output,
        "fired": decision.fired,
        "value": decision.value,
        "reason": decision.reason,
    });
    let event = ControlEvent::new(INSIGHT_EVENT_TYPE, payload, correlation.clone());
    publish(bus, event)
}
