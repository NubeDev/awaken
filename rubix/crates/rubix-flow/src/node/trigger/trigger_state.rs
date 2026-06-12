//! Process-global state for self-paced `trigger` nodes. Each interval scheduler
//! tick rebuilds the board's network and actor instances fresh, so a trigger's
//! period clock, toggle level, fire count, and boot flag cannot live in the
//! actor — they reset every tick. This registry holds that state outside the
//! per-tick actor, keyed by node id, surviving across ticks for the life of the
//! server process. A server restart clears it: that is what makes `boot` true
//! again on the first fire after start.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// One trigger node's retained state between fires.
#[derive(Debug, Clone, Copy)]
pub struct TriggerSlot {
    /// When the node last fired. `None` until the first fire — the first
    /// invocation always fires (and is the `boot` fire).
    pub last_fire: Option<Instant>,
    /// Total fires since this slot was created (server start).
    pub count: i64,
    /// The toggle level, flipped on every fire. Starts `false`, so the first
    /// fire emits `true`.
    pub level: bool,
}

impl TriggerSlot {
    const fn new() -> Self {
        Self {
            last_fire: None,
            count: 0,
            level: false,
        }
    }
}

/// The outcome of advancing a trigger slot for one invocation.
pub struct TriggerFire {
    /// `true` only on the first fire after server start (the boot fire).
    pub boot: bool,
    /// Fire count after this fire.
    pub count: i64,
    /// Toggle level after this fire.
    pub level: bool,
}

fn registry() -> &'static Mutex<HashMap<String, TriggerSlot>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, TriggerSlot>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Advance the slot for `node_id` given the elapsed-time `period`, using `now`
/// as the clock reading. Returns `Some(TriggerFire)` when `period` has elapsed
/// since the last fire (or on the very first invocation), else `None` — the
/// node stays quiet until its own period elapses, independent of how fast the
/// scheduler ticks it. The clock is injected so the logic is unit-testable.
pub fn advance(node_id: &str, period: Duration, now: Instant) -> Option<TriggerFire> {
    let mut reg = registry().lock().expect("trigger registry poisoned");
    let slot = reg
        .entry(node_id.to_string())
        .or_insert_with(TriggerSlot::new);

    let due = match slot.last_fire {
        None => true,
        Some(last) => now.duration_since(last) >= period,
    };
    if !due {
        return None;
    }

    let boot = slot.last_fire.is_none();
    slot.last_fire = Some(now);
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

    fn unique_id(tag: &str) -> String {
        // Test ids must not collide with the process-global registry across
        // tests; tag each by call site.
        format!("test-{tag}")
    }

    #[test]
    fn first_invocation_is_the_boot_fire() {
        let id = unique_id("boot");
        let now = Instant::now();
        let fire = advance(&id, Duration::from_secs(10), now).expect("first fire");
        assert!(fire.boot, "first fire is the boot fire");
        assert_eq!(fire.count, 1);
        assert!(fire.level, "level toggles to true on first fire");
    }

    #[test]
    fn quiet_until_period_elapses_then_fires_without_boot() {
        let id = unique_id("period");
        let start = Instant::now();
        let period = Duration::from_secs(10);

        assert!(advance(&id, period, start).is_some(), "boot fire");
        // Before the period elapses: no fire.
        assert!(advance(&id, period, start + Duration::from_secs(3)).is_none());
        assert!(advance(&id, period, start + Duration::from_secs(9)).is_none());
        // At/after the period: fires, not a boot fire, count advances, toggles.
        let fire = advance(&id, period, start + Duration::from_secs(10)).expect("second fire");
        assert!(!fire.boot);
        assert_eq!(fire.count, 2);
        assert!(!fire.level, "level toggles back to false on second fire");
    }
}
