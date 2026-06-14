//! Publish a control event to the in-process plane.

use crate::event::ControlEvent;

use super::registry::ControlBus;

/// Publish `event` to every current subscriber of its event type.
///
/// Returns the number of subscribers the event reached. Publishing to a type
/// with no subscribers is a no-op that returns `0`, not an error — the control
/// plane decouples components, so a publisher never depends on a listener
/// existing (`rubix/docs/SCOPE.md`, "Event bus"). The event fans out only to
/// subscribers of its own type.
pub fn publish(bus: &ControlBus, event: ControlEvent) -> usize {
    let sender = bus.sender(event.event_type());
    // `send` errors only when there are zero receivers; that is the no-op case,
    // reported as a reach count of zero rather than surfaced as a failure.
    sender.send(event).unwrap_or(0)
}
