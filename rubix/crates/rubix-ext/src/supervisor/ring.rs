//! Bounded per-extension event ring — free runtime diagnostics.
//!
//! Ported from `starter-ext-supervisor::event_ring` (`rubix/docs/design/
//! EXTENSION-RUNTIME.md`, "Status & metrics projection"). A bounded ring of
//! typed [`Event`]s per extension capturing state transitions, spawn/exit, crash
//! reasons, restart scheduling, and the last N stderr lines — surfaced at
//! `GET /extensions/<id>/events`. No IO on the hot path; appends are O(1) behind
//! a `Mutex` so the I/O reader, health pinger, and exit-watcher can all record
//! concurrently.
//!
//! The ring is intentionally typed (an [`EventKind`] enum) rather than free
//! `String`s so the admin endpoint can filter — an operator diagnosing a crash
//! loop wants "only the `Crashed` events", not a grep over prose.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::state::LifecycleState;

/// Kind of event recorded in the ring. Free to extend additively — admin UIs
/// that don't know a new kind render it generically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum EventKind {
    /// Lifecycle transition the supervisor published.
    StateTransition {
        /// The new lifecycle state.
        to: LifecycleState,
    },
    /// Child process spawned successfully. Carries the OS pid.
    Spawned {
        /// OS pid of the spawned child.
        pid: u32,
    },
    /// Child exited cleanly (exit code 0 or otherwise normal).
    ExitedClean {
        /// Exit code if observable; `None` for signal-terminated children.
        code: Option<i32>,
    },
    /// Child crashed. Reason is free-form — "non-zero exit", "health timeout",
    /// "spawn refused".
    Crashed {
        /// Human-readable reason, surfaced verbatim.
        reason: String,
    },
    /// Supervisor scheduled a restart with a wait window.
    RestartScheduled {
        /// Wait before the next spawn, in milliseconds.
        wait_ms: u64,
        /// Cumulative restart count.
        total: u64,
    },
    /// Restart intensity cap exceeded; the supervisor will not restart again.
    RestartCapExceeded {
        /// Restarts seen within the cap window.
        count: u32,
    },
    /// Missed health ping; treated as a crash by the restart tracker.
    HealthTimeout,
    /// Forwarded stderr line (trimmed of trailing newlines, capped).
    Stderr {
        /// The stderr line.
        line: String,
    },
}

/// Maximum characters captured for one [`EventKind::Stderr`] event. A child that
/// streams megabytes of stack traces still fits inside the ring.
pub const MAX_STDERR_LINE_BYTES: usize = 1024;

/// One ring entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    /// Wall-clock time the event was recorded.
    pub at: SystemTime,
    /// Monotone per-ring sequence number. Surfaced so
    /// `GET /extensions/<id>/events?after=<seq>` can resume cleanly across
    /// reconnects without depending on wall-clock equality. Never re-used — the
    /// counter is monotone even when older entries fall off the front.
    pub seq: u64,
    /// The typed event payload.
    pub kind: EventKind,
}

/// Default ring capacity (matches starter's "default 1000 entries").
pub const DEFAULT_CAPACITY: usize = 1000;

/// Bounded ring buffer. Cheap to snapshot; appends are O(1) amortised behind a
/// `Mutex`.
#[derive(Debug)]
pub struct EventRing {
    inner: Mutex<RingInner>,
    capacity: usize,
}

#[derive(Debug)]
struct RingInner {
    queue: VecDeque<Event>,
    /// Total pushes ever seen (monotone; survives ring eviction).
    next_seq: u64,
}

impl EventRing {
    /// Build a ring with the [`DEFAULT_CAPACITY`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Build a ring with a custom capacity (minimum 1 — a zero-capacity ring
    /// would silently drop every push).
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Mutex::new(RingInner {
                queue: VecDeque::with_capacity(cap.max(1)),
                next_seq: 0,
            }),
            capacity: cap.max(1),
        }
    }

    /// Append. Drops the oldest entry once the ring is full.
    pub fn push(&self, kind: EventKind) {
        let mut inner = self.inner.lock().expect("event ring mutex poisoned");
        let seq = inner.next_seq;
        inner.next_seq = inner.next_seq.wrapping_add(1);
        let event = Event {
            at: SystemTime::now(),
            seq,
            kind,
        };
        if inner.queue.len() == self.capacity {
            inner.queue.pop_front();
        }
        inner.queue.push_back(event);
    }

    /// Snapshot every entry, oldest first.
    #[must_use]
    pub fn snapshot(&self) -> Vec<Event> {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .queue
            .iter()
            .cloned()
            .collect()
    }

    /// Snapshot every entry whose `seq` is strictly greater than `after`, oldest
    /// first. Used by the live-tail SSE upgrade to resume from a cursor.
    #[must_use]
    pub fn since(&self, after: u64) -> Vec<Event> {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .queue
            .iter()
            .filter(|e| e.seq > after)
            .cloned()
            .collect()
    }

    /// The sequence number the *next* push will receive (== total pushes seen).
    #[must_use]
    pub fn next_seq(&self) -> u64 {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .next_seq
    }

    /// Entries currently in the ring.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .queue
            .len()
    }

    /// `true` when no events have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Entries evicted from the bounded ring over its lifetime — total pushes
    /// seen minus what is still retained. Surfaced as `events_dropped_total`.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        let inner = self.inner.lock().expect("event ring mutex poisoned");
        inner.next_seq.saturating_sub(inner.queue.len() as u64)
    }

    /// Count of retained [`EventKind::RestartScheduled`] events — the basis for
    /// `restarts_total`.
    #[must_use]
    pub fn restarts_total(&self) -> u64 {
        self.inner
            .lock()
            .expect("event ring mutex poisoned")
            .queue
            .iter()
            .filter(|e| matches!(e.kind, EventKind::RestartScheduled { .. }))
            .count() as u64
    }
}

impl Default for EventRing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_drops_oldest_at_capacity() {
        let ring = EventRing::with_capacity(3);
        for i in 0..5 {
            ring.push(EventKind::Spawned { pid: i });
        }
        let snap = ring.snapshot();
        assert_eq!(snap.len(), 3);
        assert!(matches!(snap[0].kind, EventKind::Spawned { pid: 2 }));
        assert!(matches!(snap[2].kind, EventKind::Spawned { pid: 4 }));
    }

    #[test]
    fn state_transition_round_trips_json() {
        let e = EventKind::StateTransition {
            to: LifecycleState::Running,
        };
        let j = serde_json::to_value(&e).unwrap();
        assert_eq!(j["kind"], "state_transition");
        let back: EventKind = serde_json::from_value(j).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn seq_is_monotone_and_survives_eviction() {
        let ring = EventRing::with_capacity(3);
        for i in 0..5 {
            ring.push(EventKind::Spawned { pid: i });
        }
        let snap = ring.snapshot();
        assert_eq!(snap[0].seq, 2);
        assert_eq!(snap[2].seq, 4);
        assert_eq!(ring.next_seq(), 5);
        assert_eq!(ring.since(2).len(), 2);
        assert_eq!(ring.since(4).len(), 0);
        assert_eq!(ring.dropped(), 2);
    }

    #[test]
    fn restarts_total_counts_restart_events() {
        let ring = EventRing::with_capacity(10);
        ring.push(EventKind::Spawned { pid: 1 });
        ring.push(EventKind::RestartScheduled {
            wait_ms: 100,
            total: 1,
        });
        ring.push(EventKind::RestartScheduled {
            wait_ms: 200,
            total: 2,
        });
        assert_eq!(ring.restarts_total(), 2);
    }
}
