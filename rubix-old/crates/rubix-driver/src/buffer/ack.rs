//! Write ack protocol. A priority-array `write` command is reliable: the sender
//! holds it in a bounded [`super::ReliableQueue`] and retries until the
//! responder acknowledges application, or a declared retry limit is reached — at
//! which point the command is given up on and the failure surfaces as a
//! [`DriverError::AckTimeout`] (no infinite retry, no silent loss). This module
//! owns the per-command retry bookkeeping and the decision of what to do next;
//! the IO (the actual `get`/reply over zenoh) stays in the driver.

use crate::error::DriverError;

/// One reliable write awaiting acknowledgement, with its retry bookkeeping. The
/// payload `T` is the command the driver re-sends on each attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingWrite<T> {
    key: String,
    command: T,
    attempts: u32,
    max_attempts: u32,
}

/// What the sender should do after an attempt or an ack, decided from the
/// command's retry state. Keeps the policy out of the IO loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAction {
    /// The command may be (re)sent; attempts remain.
    Send,
    /// The retry budget is exhausted; give up and surface an error.
    GiveUp,
}

impl<T> PendingWrite<T> {
    /// A new pending write for `key` that may be attempted up to `max_attempts`
    /// times (clamped to at least 1) before giving up.
    pub fn new(key: impl Into<String>, command: T, max_attempts: u32) -> Self {
        Self {
            key: key.into(),
            command,
            attempts: 0,
            max_attempts: max_attempts.max(1),
        }
    }

    /// The command to send, for the driver's IO to publish/query.
    pub fn command(&self) -> &T {
        &self.command
    }

    /// The keyexpr this write targets.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Attempts made so far.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Record that an attempt was made and report whether more remain. Call this
    /// immediately before (or after) each send; once attempts reach the limit it
    /// returns [`WriteAction::GiveUp`].
    pub fn record_attempt(&mut self) -> WriteAction {
        self.attempts += 1;
        if self.attempts >= self.max_attempts {
            WriteAction::GiveUp
        } else {
            WriteAction::Send
        }
    }

    /// True if the retry budget is exhausted.
    pub fn exhausted(&self) -> bool {
        self.attempts >= self.max_attempts
    }

    /// Build the give-up error for this command, naming the key and attempt
    /// count, so the failure surfaces to the caller as a spark/error.
    pub fn give_up_error(&self) -> DriverError {
        DriverError::AckTimeout {
            key: self.key.clone(),
            attempts: self.attempts,
        }
    }
}

/// The responder's acknowledgement of a write: whether the priority-array write
/// was applied, with an optional reason when it was not. Serializable so it can
/// travel as the query reply payload on the `write` channel.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WriteAck {
    /// True when the responder applied the command to the priority array.
    pub applied: bool,
    /// Human-readable reason when `applied` is false (e.g. a higher-priority
    /// override holds the slot). `None` on success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl WriteAck {
    /// An ack confirming the write was applied.
    pub fn applied() -> Self {
        Self {
            applied: true,
            reason: None,
        }
    }

    /// A negative ack: the responder is alive and answered, but did not apply
    /// the write, for the given reason. A negative ack is a definitive outcome,
    /// not a reason to retry — retries are for *missing* acks (timeouts).
    pub fn rejected(reason: impl Into<String>) -> Self {
        Self {
            applied: false,
            reason: Some(reason.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_then_gives_up_at_the_limit() {
        let mut w = PendingWrite::new("nube/hq/ahu-3/temp/write", "set 21.0", 3);
        assert_eq!(w.record_attempt(), WriteAction::Send); // 1
        assert_eq!(w.record_attempt(), WriteAction::Send); // 2
        assert_eq!(w.record_attempt(), WriteAction::GiveUp); // 3 == limit
        assert!(w.exhausted());
        assert_eq!(
            w.give_up_error(),
            DriverError::AckTimeout {
                key: "nube/hq/ahu-3/temp/write".into(),
                attempts: 3,
            }
        );
    }

    #[test]
    fn single_attempt_gives_up_immediately() {
        let mut w = PendingWrite::new("k/write", (), 1);
        assert_eq!(w.record_attempt(), WriteAction::GiveUp);
        assert_eq!(w.command(), &());
        assert_eq!(w.key(), "k/write");
    }

    #[test]
    fn ack_round_trips_as_json() {
        let applied = WriteAck::applied();
        let json = serde_json::to_string(&applied).unwrap();
        assert_eq!(json, r#"{"applied":true}"#);
        assert_eq!(serde_json::from_str::<WriteAck>(&json).unwrap(), applied);

        let nak = WriteAck::rejected("priority 8 override holds the slot");
        let back: WriteAck = serde_json::from_str(&serde_json::to_string(&nak).unwrap()).unwrap();
        assert_eq!(back, nak);
        assert!(!back.applied);
    }
}
