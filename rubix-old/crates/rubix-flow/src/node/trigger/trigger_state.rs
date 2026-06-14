//! The retained state of a self-paced `trigger` node, and the pure clock logic
//! that advances it. The slot is JSON (it round-trips through [`crate::NodeState`]
//! under the node's chosen [`crate::StatePolicy`]), so timing uses a wall-clock
//! millis stamp rather than a monotonic `Instant`; acceptable for the
//! supervisory sec/min/hour cadences a `trigger` paces.

use serde::{Deserialize, Serialize};

/// One trigger node's retained state between fires.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TriggerSlot {
    /// Wall-clock millis of the last fire. `None` until the first fire — the
    /// first invocation always fires (and is the `boot` fire).
    pub last_fire_ms: Option<i64>,
    /// Total fires since this slot was created.
    pub count: i64,
    /// The toggle level, flipped on every fire. Starts `false`, so the first
    /// fire emits `true`.
    pub level: bool,
}

/// The outcome of advancing a trigger slot for one invocation.
pub struct TriggerFire {
    /// `true` only on the first fire of this slot (the boot fire).
    pub boot: bool,
    /// Fire count after this fire.
    pub count: i64,
    /// Toggle level after this fire.
    pub level: bool,
}

/// Advance `slot` for one invocation at wall-clock `now_ms`, given the elapsed
/// `period_ms`. Returns `Some(TriggerFire)` when `period_ms` has elapsed since
/// the last fire (or on the very first invocation), else `None` — the node stays
/// quiet until its own period elapses, independent of how fast it is ticked. Pure
/// (clock injected), so it is unit-testable.
pub fn advance(slot: &mut TriggerSlot, period_ms: i64, now_ms: i64) -> Option<TriggerFire> {
    let due = match slot.last_fire_ms {
        None => true,
        Some(last) => now_ms.saturating_sub(last) >= period_ms,
    };
    if !due {
        return None;
    }
    let boot = slot.last_fire_ms.is_none();
    slot.last_fire_ms = Some(now_ms);
    slot.count += 1;
    slot.level = !slot.level;
    Some(TriggerFire {
        boot,
        count: slot.count,
        level: slot.level,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_invocation_is_the_boot_fire() {
        let mut slot = TriggerSlot::default();
        let fire = advance(&mut slot, 10_000, 0).expect("first fire");
        assert!(fire.boot, "first fire is the boot fire");
        assert_eq!(fire.count, 1);
        assert!(fire.level, "level toggles to true on first fire");
    }

    #[test]
    fn quiet_until_period_elapses_then_fires_without_boot() {
        let mut slot = TriggerSlot::default();
        let period = 10_000;

        assert!(advance(&mut slot, period, 0).is_some(), "boot fire");
        assert!(advance(&mut slot, period, 3_000).is_none());
        assert!(advance(&mut slot, period, 9_000).is_none());
        let fire = advance(&mut slot, period, 10_000).expect("second fire");
        assert!(!fire.boot);
        assert_eq!(fire.count, 2);
        assert!(!fire.level, "level toggles back to false on second fire");
    }
}
