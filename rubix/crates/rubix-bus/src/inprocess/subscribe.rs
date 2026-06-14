//! Subscribe to one event type on the in-process plane.

use tokio::sync::broadcast;

use crate::event::ControlEvent;

use super::registry::ControlBus;

/// A subscription to one event type's control events.
///
/// Wraps a tokio broadcast receiver. Each call to [`ControlSubscription::recv`]
/// yields the next event of the subscribed type; events published before the
/// subscription opened are not replayed (the plane is live coordination, not a
/// log).
pub struct ControlSubscription {
    receiver: broadcast::Receiver<ControlEvent>,
}

impl ControlSubscription {
    /// Await the next control event of the subscribed type.
    ///
    /// # Errors
    /// Returns a [`broadcast::error::RecvError`]: `Closed` once every publisher
    /// for the type has been dropped, or `Lagged(n)` if this subscriber fell
    /// `n` events behind the bounded buffer and the oldest were overwritten.
    pub async fn recv(&mut self) -> std::result::Result<ControlEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }
}

/// Subscribe to `event_type`, receiving every event of that type published
/// after this call.
///
/// Subscribing creates the type's channel if it does not yet exist, so a
/// subscriber may register before any publisher. A subscription to one type
/// never observes another type's events.
#[must_use]
pub fn subscribe(bus: &ControlBus, event_type: &str) -> ControlSubscription {
    ControlSubscription {
        receiver: bus.sender(event_type).subscribe(),
    }
}
