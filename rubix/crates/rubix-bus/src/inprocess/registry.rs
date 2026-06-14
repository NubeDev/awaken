//! The in-process broadcast registry: one tokio broadcast channel per event
//! type.
//!
//! The control plane fans an event out to every live subscriber of its type and
//! to no one else (`rubix/docs/SCOPE.md`, "Event bus" — in-process control). A
//! subscriber to `record.created` never receives `rule.fired`, because each
//! type owns its own channel. The registry is cloneable and shared across the
//! binary's components; cloning is an `Arc` bump.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::event::ControlEvent;

/// The per-event-type broadcast channel capacity.
///
/// A bounded ring buffer keeps a slow subscriber from growing memory without
/// limit; an overrun is reported to that subscriber as a lag, not a panic
/// (`rubix/docs/SCOPE.md`, "Event bus"). The control plane carries low-volume
/// component coordination, so a modest buffer is ample.
const CHANNEL_CAPACITY: usize = 256;

/// A cloneable handle to the in-process control plane.
///
/// Holds one tokio broadcast sender per event type, created lazily on first
/// publish or subscribe to that type. The map is guarded by a `Mutex` only for
/// the brief registry lookup; the broadcast send/receive itself is lock-free.
#[derive(Clone, Default)]
pub struct ControlBus {
    channels: Arc<Mutex<HashMap<String, broadcast::Sender<ControlEvent>>>>,
}

impl ControlBus {
    /// Create an empty control bus.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the sender for `event_type`, creating its channel on first use.
    ///
    /// The lock is held only for the map lookup/insert, never across a send.
    pub(crate) fn sender(&self, event_type: &str) -> broadcast::Sender<ControlEvent> {
        // Recover the guard if a prior holder panicked: the registry is a plain
        // type→sender map, never left half-updated mid-lookup, so a poisoned
        // lock carries no corrupt state and fan-out must keep working.
        let mut channels = self
            .channels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        channels
            .entry(event_type.to_owned())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .clone()
    }
}
