//! The per-extension counter registry — the leaf the metrics view folds in.
//!
//! Ported from `starter-ext-metrics` (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! phase 3; "Adopt — leaf crate"), kept here as a module so the runtime half is
//! one crate. It holds nothing but a map of [`ExtensionId`] → atomic
//! [`Counters`]. The dependency arrows point one way: the planes that bump a
//! counter (the control plane on each gated command, the bus plane on each
//! publish/receive) take a cheap `&MetricsRegistry`; the
//! [`supervisor`](crate::supervisor) supplies process gauges; the admin surface
//! folds the two into one [`ExtensionMetrics`] via [`MetricsRegistry::merged`].
//!
//! Unlike starter this uses a `Mutex<HashMap>` rather than `dashmap` — the
//! runtime must not pull a new workspace dependency in for a handful of atomics,
//! and the registry is read on the (cold) admin path and bumped on planes that
//! already cross the gate, so contention is a non-issue.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::supervisor::{ExtensionId, LifecycleState, ProcessStats};

/// The atomic counters one extension accumulates over the host's lifetime. Every
/// field is monotone. Snapshotted (not moved) when building the merged view, so
/// the live tallies keep counting.
#[derive(Debug, Default)]
pub struct Counters {
    commands: AtomicU64,
    command_errors: AtomicU64,
    events_published: AtomicU64,
    events_received: AtomicU64,
}

impl Counters {
    /// Record a gated control command applied for this extension.
    #[inline]
    pub fn record_command(&self) {
        self.commands.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a gated command that failed at apply. Bumped *in addition to*
    /// [`Self::record_command`], so `command_errors` is a subset of `commands`.
    #[inline]
    pub fn record_command_error(&self) {
        self.command_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a control event the extension published onto the bus.
    #[inline]
    pub fn record_event_published(&self) {
        self.events_published.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a control event delivered to the extension's subscription.
    #[inline]
    pub fn record_event_received(&self) {
        self.events_received.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            commands: self.commands.load(Ordering::Relaxed),
            command_errors: self.command_errors.load(Ordering::Relaxed),
            events_published: self.events_published.load(Ordering::Relaxed),
            events_received: self.events_received.load(Ordering::Relaxed),
        }
    }
}

/// A point-in-time read of one extension's [`Counters`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CounterSnapshot {
    /// Gated commands applied.
    pub commands: u64,
    /// Gated commands that failed at apply (subset of `commands`).
    pub command_errors: u64,
    /// Control events published onto the bus.
    pub events_published: u64,
    /// Control events delivered to the extension.
    pub events_received: u64,
}

/// The process-side gauges the supervisor supplies when merging. A plain value
/// type so this module never depends on the supervisor's internals — the caller
/// fills it from a [`SupervisorHandle`](crate::supervisor::SupervisorHandle).
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessGauges {
    /// Sampled process stats; `None` for builtin/wasm or when not running.
    pub process: Option<ProcessStats>,
    /// Current lifecycle state.
    pub lifecycle_state: LifecycleState,
    /// Cumulative restarts.
    pub restarts_total: u64,
    /// Cumulative capability violations (fail-closed gate denials) attributed to
    /// this extension — authorization health, not just process health
    /// (`rubix/docs/design/EXTENSION-RUNTIME.md`, "the one rubix-specific add").
    pub capability_violations_total: u64,
    /// Event-ring evictions.
    pub events_dropped_total: u64,
}

/// The merged metrics view for a single extension, served by
/// `GET /extensions/<id>/metrics`. Counters are monotone since host start;
/// gauges reflect the latest sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionMetrics {
    /// Sampled process stats; `None` for builtin/wasm or not-`Running`.
    pub process: Option<ProcessStats>,
    /// Current lifecycle state.
    pub lifecycle_state: LifecycleState,
    /// Cumulative restarts the supervisor has performed.
    pub restarts_total: u64,
    /// Cumulative capability violations refused fail closed.
    pub capability_violations_total: u64,
    /// Gated control commands applied for this extension.
    pub commands_total: u64,
    /// Gated commands that failed at apply (subset of `commands_total`).
    pub command_errors_total: u64,
    /// Control events the extension published onto the bus.
    pub events_published_total: u64,
    /// Control events delivered to the extension's subscription.
    pub events_received_total: u64,
    /// Event-ring evictions (entries pushed out of the bounded ring).
    pub events_dropped_total: u64,
}

/// A cheap-to-clone handle to the per-extension counter map. Planes keep a clone
/// and bump on their path; the admin surface keeps a clone and calls
/// [`Self::merged`] to build the response.
#[derive(Clone, Default)]
pub struct MetricsRegistry {
    inner: Arc<Mutex<HashMap<ExtensionId, Arc<Counters>>>>,
}

impl std::fmt::Debug for MetricsRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.inner.lock().map(|g| g.len()).unwrap_or(0);
        f.debug_struct("MetricsRegistry")
            .field("tracked", &len)
            .finish()
    }
}

impl MetricsRegistry {
    /// A fresh, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get (creating if absent) the [`Counters`] for an extension. The returned
    /// `Arc` can be held by a plane so subsequent bumps skip the map lookup.
    #[must_use]
    pub fn counters(&self, id: &ExtensionId) -> Arc<Counters> {
        let mut guard = self.inner.lock().expect("metrics registry poisoned");
        Arc::clone(
            guard
                .entry(id.clone())
                .or_insert_with(|| Arc::new(Counters::default())),
        )
    }

    /// Snapshot one extension's counters, or all-zero when never recorded.
    #[must_use]
    pub fn snapshot(&self, id: &ExtensionId) -> CounterSnapshot {
        self.inner
            .lock()
            .expect("metrics registry poisoned")
            .get(id)
            .map(|c| c.snapshot())
            .unwrap_or_default()
    }

    /// Fold the counters with the supervisor-supplied process gauges into the
    /// merged [`ExtensionMetrics`] view. The single projection point the
    /// `GET /extensions/<id>/metrics` handler calls.
    #[must_use]
    pub fn merged(&self, id: &ExtensionId, gauges: ProcessGauges) -> ExtensionMetrics {
        let c = self.snapshot(id);
        ExtensionMetrics {
            process: gauges.process,
            lifecycle_state: gauges.lifecycle_state,
            restarts_total: gauges.restarts_total,
            capability_violations_total: gauges.capability_violations_total,
            commands_total: c.commands,
            command_errors_total: c.command_errors,
            events_published_total: c.events_published,
            events_received_total: c.events_received,
            events_dropped_total: gauges.events_dropped_total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> ExtensionId {
        ExtensionId::new("rubix", "demo-ext")
    }

    #[test]
    fn counters_increment_independently() {
        let reg = MetricsRegistry::new();
        let c = reg.counters(&id());
        c.record_command();
        c.record_command();
        c.record_command_error();
        c.record_event_published();
        c.record_event_received();
        c.record_event_received();

        let s = reg.snapshot(&id());
        assert_eq!(s.commands, 2);
        assert_eq!(s.command_errors, 1);
        assert_eq!(s.events_published, 1);
        assert_eq!(s.events_received, 2);
    }

    #[test]
    fn counters_handle_is_shared_not_copied() {
        let reg = MetricsRegistry::new();
        reg.counters(&id()).record_command();
        reg.counters(&id()).record_command();
        assert_eq!(reg.snapshot(&id()).commands, 2);
    }

    #[test]
    fn unknown_extension_snapshots_to_zero() {
        let reg = MetricsRegistry::new();
        assert_eq!(reg.snapshot(&id()), CounterSnapshot::default());
    }

    #[test]
    fn merged_projection_combines_both_sources() {
        let reg = MetricsRegistry::new();
        let c = reg.counters(&id());
        c.record_command();
        c.record_command();
        c.record_command_error();
        c.record_event_published();

        let merged = reg.merged(
            &id(),
            ProcessGauges {
                process: None,
                lifecycle_state: LifecycleState::Running,
                restarts_total: 3,
                capability_violations_total: 2,
                events_dropped_total: 7,
            },
        );

        assert_eq!(merged.commands_total, 2);
        assert_eq!(merged.command_errors_total, 1);
        assert_eq!(merged.events_published_total, 1);
        assert_eq!(merged.lifecycle_state, LifecycleState::Running);
        assert_eq!(merged.restarts_total, 3);
        assert_eq!(merged.capability_violations_total, 2);
        assert_eq!(merged.events_dropped_total, 7);
        assert!(merged.process.is_none());
    }

    #[test]
    fn merged_round_trips_json() {
        let reg = MetricsRegistry::new();
        let m = reg.merged(
            &id(),
            ProcessGauges {
                process: None,
                lifecycle_state: LifecycleState::Stopped,
                restarts_total: 0,
                capability_violations_total: 0,
                events_dropped_total: 0,
            },
        );
        let j = serde_json::to_value(&m).unwrap();
        assert_eq!(j["lifecycle_state"], "stopped");
        assert!(j["process"].is_null());
        let back: ExtensionMetrics = serde_json::from_value(j).unwrap();
        assert_eq!(back, m);
    }
}
