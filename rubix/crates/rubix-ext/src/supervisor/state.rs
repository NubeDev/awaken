//! [`LifecycleState`] — the runtime state of a supervised extension.
//!
//! Ported from `starter-ext-spi`'s `LifecycleState` (`rubix/docs/design/
//! EXTENSION-RUNTIME.md`, "Adopt the shapes"). One vocabulary shared by the
//! supervisor's state machine, the event ring, and the admin projections so a
//! dashboard, `GET /extensions/<id>`, and the ring never disagree on what
//! "running" means.
//!
//! This is the *observed* runtime state of a child process — distinct from the
//! gated [`LifecycleAction`](crate::LifecycleAction) an operator *requests*. A
//! `start` action drives the supervisor toward [`Running`](LifecycleState::Running);
//! the supervisor reports back where it actually got to.

use serde::{Deserialize, Serialize};

/// The observed runtime state of a supervised extension's child process.
///
/// The variants form a small state machine, not an arbitrary set:
///
/// - `Starting` → `Running` (init handshake succeeded) **or** `Crashed`
///   (failure during spawn/init).
/// - `Running` → `Stopping` (graceful) → `Stopped`, **or** `Crashed` (abnormal
///   exit / missed health ping), **or** `Failed` (restart intensity cap
///   exceeded; no further restart).
/// - `Stopped` → `Starting` (operator re-enabled it).
///
/// `Failed` is terminal until an operator re-enables the extension, which resets
/// the restart tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Supervisor is bringing the extension up (spawn + init handshake).
    Starting,
    /// Init handshake succeeded; the child is serving.
    Running,
    /// Graceful shutdown in progress (within the supervisor's grace window).
    Stopping,
    /// Cleanly stopped. An operator can re-enable it.
    Stopped,
    /// Abnormal exit, missed health ping, or panic; the supervisor will restart
    /// per policy unless the intensity cap is exceeded.
    Crashed,
    /// Terminal failure (restart intensity cap exceeded). No automatic restart.
    Failed,
}

impl LifecycleState {
    /// `true` if the supervisor considers this state terminal — no further
    /// transition happens without an explicit operator action.
    #[inline]
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Failed)
    }

    /// `true` if the extension is actively able to serve a request.
    #[inline]
    #[must_use]
    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_only_failed() {
        for s in [
            LifecycleState::Starting,
            LifecycleState::Running,
            LifecycleState::Stopping,
            LifecycleState::Stopped,
            LifecycleState::Crashed,
        ] {
            assert!(!s.is_terminal(), "{s:?} should not be terminal");
        }
        assert!(LifecycleState::Failed.is_terminal());
    }

    #[test]
    fn snake_case_wire_form() {
        assert_eq!(
            serde_json::to_string(&LifecycleState::Running).unwrap(),
            "\"running\""
        );
        let back: LifecycleState = serde_json::from_str("\"crashed\"").unwrap();
        assert_eq!(back, LifecycleState::Crashed);
    }
}
