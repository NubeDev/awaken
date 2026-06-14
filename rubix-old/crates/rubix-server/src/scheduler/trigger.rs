//! What fires a stored board. A board is either run on demand (`Manual`,
//! via `/boards/{slug}/run`), on a fixed cadence (`Interval`), or whenever a
//! `cur` value arrives on a subscribed keyexpr (`Subscription`). The variant
//! is persisted as the board row's `trigger` JSON column.

use serde::{Deserialize, Serialize};

/// How the scheduler decides to evaluate a board.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Trigger {
    /// Never fired by the scheduler; only `/boards/{slug}/run` runs it.
    Manual,
    /// Fire every `seconds`. Minimum one second — sub-second control loops
    /// belong on the edge driver, not the supervisory scheduler.
    Interval { seconds: u64 },
    /// Fire whenever a `cur` sample is published on a keyexpr matching
    /// `key`, which may be a zenoh wildcard (`nube/hq/*/temp/cur`).
    Subscription { key: String },
}

impl Trigger {
    /// Reject nonsensical trigger config before persistence. An interval of
    /// zero would spin; an empty subscription key matches nothing.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Trigger::Manual => Ok(()),
            Trigger::Interval { seconds } if *seconds == 0 => {
                Err("interval trigger requires seconds >= 1".into())
            }
            Trigger::Interval { .. } => Ok(()),
            Trigger::Subscription { key } if key.trim().is_empty() => {
                Err("subscription trigger requires a non-empty key".into())
            }
            Trigger::Subscription { .. } => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_zero_rejected() {
        assert!(Trigger::Interval { seconds: 0 }.validate().is_err());
        assert!(Trigger::Interval { seconds: 1 }.validate().is_ok());
    }

    #[test]
    fn empty_subscription_rejected() {
        assert!(Trigger::Subscription { key: "  ".into() }
            .validate()
            .is_err());
        assert!(Trigger::Subscription {
            key: "nube/hq/*/cur".into()
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn round_trips_tagged_json() {
        let t = Trigger::Interval { seconds: 30 };
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"kind":"interval","seconds":30}"#);
        assert_eq!(serde_json::from_str::<Trigger>(&json).unwrap(), t);
    }
}
