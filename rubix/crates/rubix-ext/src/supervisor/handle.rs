//! [`SupervisorHandle`] — the live, queryable face of a supervised extension.
//!
//! Ported from `starter-ext-supervisor::supervisor::SupervisorHandle`
//! (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase 1). The supervisor task runs
//! the spawn/serve/restart loop; this handle is what every reader holds onto —
//! the admin projections, the metrics merge, the boot reconciler. It never blocks
//! the task: state is a `watch` channel, the event ring and process cell are
//! shared `Arc`s, and shutdown is a one-shot `mpsc` signal.
//!
//! All read methods are bounded and safe on the HTTP request path; none touch the
//! database — process/metrics gauges are pure in-memory reads (`rubix/docs/
//! design/EXTENSION-RUNTIME.md`, "Reads are SurrealDB-native … gauges are read
//! off the in-memory supervisor handles").

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use tokio::sync::{mpsc, watch};

use super::id::ExtensionId;
use super::ring::{Event, EventRing};
use super::state::LifecycleState;
use super::stats::{ProcessCell, ProcessStats};

/// A cheap-to-clone handle to a running supervisor task.
#[derive(Debug, Clone)]
pub struct SupervisorHandle {
    pub(super) id: ExtensionId,
    pub(super) state: watch::Receiver<LifecycleState>,
    pub(super) shutdown_tx: mpsc::Sender<()>,
    pub(super) events: Arc<EventRing>,
    pub(super) violations: Arc<AtomicU64>,
    pub(super) inbound: mpsc::UnboundedSender<serde_json::Value>,
    pub(super) process: ProcessCell,
}

impl SupervisorHandle {
    /// The extension's id, so an admin endpoint can key by id without a lookup.
    #[must_use]
    pub fn id(&self) -> &ExtensionId {
        &self.id
    }

    /// Subscribe to lifecycle-state changes. The `watch` channel always holds the
    /// current value, so a late subscriber immediately sees the steady state.
    #[must_use]
    pub fn state(&self) -> watch::Receiver<LifecycleState> {
        self.state.clone()
    }

    /// The current lifecycle state.
    #[must_use]
    pub fn lifecycle_state(&self) -> LifecycleState {
        *self.state.borrow()
    }

    /// Snapshot the event ring. Bounded; safe on the request path.
    #[must_use]
    pub fn events(&self) -> Vec<Event> {
        self.events.snapshot()
    }

    /// Snapshot ring events whose `seq` is strictly greater than `after` — the
    /// cursor the SSE live-tail resumes from.
    #[must_use]
    pub fn events_since(&self, after: u64) -> Vec<Event> {
        self.events.since(after)
    }

    /// The cursor the next ring event will receive.
    #[must_use]
    pub fn events_next_seq(&self) -> u64 {
        self.events.next_seq()
    }

    /// The current child's OS pid, or `None` when not `Running` (starting,
    /// stopped, failed, or never spawned). Gated on the `Running` state so a
    /// stale pid is never reported during teardown.
    #[must_use]
    pub fn pid(&self) -> Option<u32> {
        if !self.lifecycle_state().is_running() {
            return None;
        }
        self.process.lock().ok()?.as_ref().map(|p| p.pid)
    }

    /// Sampled [`ProcessStats`] for the current child, or `None` when not
    /// `Running`. `rss_bytes` / `cpu_pct` are best-effort, filled by the
    /// health-tick sampler. Bounded; safe on the request path.
    #[must_use]
    pub fn process_stats(&self) -> Option<ProcessStats> {
        if !self.lifecycle_state().is_running() {
            return None;
        }
        let guard = self.process.lock().ok()?;
        guard.as_ref().map(|p| p.to_stats(Instant::now()))
    }

    /// Cumulative restarts over this handle's lifetime, derived from the
    /// `RestartScheduled` events retained in the ring. Surfaced as
    /// `restarts_total`.
    #[must_use]
    pub fn restarts_total(&self) -> u64 {
        self.events.restarts_total()
    }

    /// Events evicted from the bounded ring over this handle's lifetime.
    /// Surfaced as `events_dropped_total`.
    #[must_use]
    pub fn events_dropped(&self) -> u64 {
        self.events.dropped()
    }

    /// Capability violations attributed to this extension — the count of
    /// fail-closed gate denials the bridge has recorded against it (see
    /// [`Self::record_violation`]). Surfaced as `capability_violations_total`,
    /// so the metrics view shows authorization health, not just process health
    /// (`rubix/docs/design/EXTENSION-RUNTIME.md`, "the one rubix-specific add").
    #[must_use]
    pub fn capability_violations(&self) -> u64 {
        self.violations.load(Ordering::Relaxed)
    }

    /// Record one capability violation against this extension. Called by the
    /// bridge when the gate denies an extension command fail closed, so the
    /// violation surfaces on the metrics view alongside the process gauges.
    pub fn record_violation(&self) {
        self.violations.fetch_add(1, Ordering::Relaxed);
    }

    /// Whether the supervised child is live — the process-flavour liveness probe
    /// `POST /extensions/<id>/health` consults (`rubix/docs/design/
    /// EXTENSION-RUNTIME.md`, phase 5). A child is live exactly when the
    /// supervisor's observed state is `Running`; the init handshake completing is
    /// what flips the state, so a `true` here means the child answered its
    /// handshake and has not since crashed or been told to stop.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.lifecycle_state().is_running()
    }

    /// Send a JSON-RPC envelope to the child (request or notification). The
    /// caller constructs valid JSON-RPC; this is the outbound side of the
    /// control channel.
    ///
    /// Returns `false` if the supervisor task is no longer running.
    pub fn send(&self, envelope: serde_json::Value) -> bool {
        self.inbound.send(envelope).is_ok()
    }

    /// Request a graceful shutdown: the supervisor sends a cooperative `shutdown`
    /// notification, waits the grace window, then hard-kills if needed. Awaiting
    /// the returned future only enqueues the request; observe
    /// [`Self::state`] for the transition to `Stopped`.
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}
