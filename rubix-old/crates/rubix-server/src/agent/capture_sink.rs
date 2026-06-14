//! An [`EventSink`] that records a run's events and signals the instant the run
//! suspends.
//!
//! The embedded runtime resolves to a full-local backend, whose suspend path
//! blocks the agent loop on a live decision channel waiting for an operator
//! approve/cancel that — in this architecture — never arrives through the loop:
//! the operator surface is our own `runs` HTTP API, which re-applies the held
//! write directly (see `api/runs/resume.rs`), not by feeding a decision back
//! into awaken. So once the loop emits `RunFinish { Suspended }` it has produced
//! everything we need (the held write rides the preceding `ToolCallDone`); this
//! sink fires a [`Notify`] on that event so the caller can cancel the otherwise-
//! blocked run and release the loop. Completed runs never trip the notify and
//! finish normally.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use awaken_runtime_contract::contract::event::AgentEvent;
use awaken_runtime_contract::contract::event_sink::EventSink;
use awaken_runtime_contract::contract::lifecycle::TerminationReason;
use tokio::sync::Notify;

/// Collects events and signals on the first `RunFinish { Suspended }`, recording
/// that run's id so the caller can cancel the blocked loop.
#[derive(Default)]
pub(super) struct SuspendCaptureSink {
    events: Mutex<Vec<AgentEvent>>,
    suspended_run_id: Mutex<Option<String>>,
    suspended: Notify,
}

impl SuspendCaptureSink {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Resolves once the run suspends.
    pub(super) async fn suspended(&self) {
        self.suspended.notified().await;
    }

    /// The id of the suspended run, set once [`suspended`](Self::suspended)
    /// fires.
    pub(super) fn suspended_run_id(&self) -> Option<String> {
        lock(&self.suspended_run_id).clone()
    }

    /// Take all collected events.
    pub(super) fn take_events(&self) -> Vec<AgentEvent> {
        std::mem::take(&mut *lock(&self.events))
    }
}

/// Recover the guard past a poisoned mutex: the critical sections never panic
/// (push / take / clone), so poisoning would only stem from an unrelated abort,
/// and dropping the captured events on top of that adds no safety.
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[async_trait]
impl EventSink for SuspendCaptureSink {
    async fn emit(&self, event: AgentEvent) {
        if let AgentEvent::RunFinish {
            run_id,
            termination: TerminationReason::Suspended,
            ..
        } = &event
        {
            *lock(&self.suspended_run_id) = Some(run_id.clone());
            self.suspended.notify_one();
        }
        lock(&self.events).push(event);
    }
}
