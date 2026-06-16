//! Restart-policy state machine.
//!
//! Ported from `starter-ext-supervisor::restart` (`rubix/docs/design/
//! EXTENSION-RUNTIME.md`, phase 1: "restart with backoff"). After each child
//! exit the supervisor asks [`RestartTracker::should_restart`] and gets one of
//! three answers:
//!
//! - [`RestartDecision::Restart`] — wait the next backoff, respawn.
//! - [`RestartDecision::Stop`] — the policy says "do not restart"
//!   ([`RestartPolicy::Never`], or [`RestartPolicy::OnCrash`] with a clean exit).
//! - [`RestartDecision::Failed`] — the intensity cap is exceeded; the supervisor
//!   transitions to [`LifecycleState::Failed`](super::state::LifecycleState::Failed)
//!   and stops trying.
//!
//! The intensity cap is "at most N restarts within the last M seconds", computed
//! from `Instant::now()` against a small ring of recent restart times.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// How aggressively the supervisor restarts an extension after it exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Restart after any exit, clean or crash.
    Always,
    /// Restart only after a crash (non-zero exit, signal, missed health ping).
    /// A clean exit is left stopped.
    #[default]
    OnCrash,
    /// Never restart; the supervisor stops after the first exit.
    Never,
}

/// Reason the child exited; drives the [`RestartPolicy::OnCrash`] choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// Child exited cleanly (code 0).
    Clean,
    /// Child crashed (signal, non-zero exit, missed health ping, …).
    Crash,
}

/// The supervisor's next move after a child exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartDecision {
    /// Spawn again after the next backoff step.
    Restart,
    /// Honour the policy and settle on `Stopped`. The supervisor task exits.
    Stop,
    /// Intensity cap exceeded; transition to `Failed`.
    Failed,
}

/// Per-extension restart tracker.
#[derive(Debug, Clone)]
pub struct RestartTracker {
    policy: RestartPolicy,
    max_restarts: u32,
    window: Duration,
    recent: VecDeque<Instant>,
    total: u64,
}

impl RestartTracker {
    /// Build a tracker for `policy`, capping at `max_restarts` within
    /// `within_seconds`.
    #[must_use]
    pub fn new(policy: RestartPolicy, max_restarts: u32, within_seconds: u32) -> Self {
        Self {
            policy,
            max_restarts,
            window: Duration::from_secs(u64::from(within_seconds.max(1))),
            recent: VecDeque::new(),
            total: 0,
        }
    }

    /// Total restarts since this tracker was created.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Decide what to do after a child exit. Records the restart attempt into
    /// the sliding window when the decision is `Restart`.
    pub fn should_restart(&mut self, reason: ExitReason) -> RestartDecision {
        let want_restart = match (self.policy, reason) {
            (RestartPolicy::Always, _) => true,
            (RestartPolicy::OnCrash, ExitReason::Crash) => true,
            (RestartPolicy::OnCrash, ExitReason::Clean) => false,
            (RestartPolicy::Never, _) => false,
        };
        if !want_restart {
            return RestartDecision::Stop;
        }

        let now = Instant::now();
        self.prune(now);
        if self.recent.len() as u32 >= self.max_restarts {
            return RestartDecision::Failed;
        }
        self.recent.push_back(now);
        self.total = self.total.saturating_add(1);
        RestartDecision::Restart
    }

    fn prune(&mut self, now: Instant) {
        while let Some(front) = self.recent.front() {
            if now.duration_since(*front) > self.window {
                self.recent.pop_front();
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_policy_always_stops() {
        let mut t = RestartTracker::new(RestartPolicy::Never, 5, 60);
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Stop);
        assert_eq!(t.should_restart(ExitReason::Clean), RestartDecision::Stop);
    }

    #[test]
    fn on_crash_skips_clean_exit() {
        let mut t = RestartTracker::new(RestartPolicy::OnCrash, 5, 60);
        assert_eq!(t.should_restart(ExitReason::Clean), RestartDecision::Stop);
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Restart);
    }

    #[test]
    fn intensity_cap_transitions_to_failed() {
        let mut t = RestartTracker::new(RestartPolicy::Always, 3, 60);
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Restart);
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Restart);
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Restart);
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Failed);
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Failed);
        assert_eq!(t.total(), 3);
    }
}
