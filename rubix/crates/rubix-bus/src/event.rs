//! The in-process control-event envelope.
//!
//! The control plane carries component-to-component events inside the binary
//! with no serialization and no network (`rubix/docs/SCOPE.md`, "Event bus" —
//! in-process tokio channels). An event is a type tag, a free-form JSON payload,
//! and the correlation id that threads it to the action that produced it
//! (contract #3, `rubix/STACK-DEISGN.md`). Subscribers filter by event type, so
//! a subscriber to one type never receives another.
//!
//! The data-change plane has its own event shape — it carries a domain
//! [`Record`](rubix_core::Record) and the change action — and lives in
//! [`crate::livequery`].

use rubix_core::CorrelationId;

/// A control event published on the in-process plane.
///
/// `Clone` is required because the underlying tokio broadcast fans the event
/// out to every subscriber by value; the payload is plain JSON so the clone is
/// a deep copy with no shared-state surprises.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlEvent {
    /// The event type tag subscribers filter on.
    event_type: String,
    /// The free-form payload — the bus imposes no shape on it.
    payload: serde_json::Value,
    /// The correlation id threading this event to its originating action.
    correlation_id: CorrelationId,
}

impl ControlEvent {
    /// Build a control event of `event_type` carrying `payload`, threaded by
    /// `correlation_id`.
    #[must_use]
    pub fn new(
        event_type: impl Into<String>,
        payload: serde_json::Value,
        correlation_id: CorrelationId,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
            correlation_id,
        }
    }

    /// The event type tag.
    #[must_use]
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// The event payload.
    #[must_use]
    pub fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    /// The correlation id threading this event to its originating action.
    #[must_use]
    pub fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::CorrelationId;

    use super::ControlEvent;

    #[test]
    fn carries_type_payload_and_correlation() {
        let corr = CorrelationId::carry("corr-1");
        let event = ControlEvent::new(
            "record.created",
            serde_json::json!({ "id": "r1" }),
            corr.clone(),
        );
        assert_eq!(event.event_type(), "record.created");
        assert_eq!(event.payload(), &serde_json::json!({ "id": "r1" }));
        assert_eq!(event.correlation_id(), &corr);
    }
}
