//! Exponential restart backoff with optional jitter.
//!
//! Ported from `starter-ext-supervisor::backoff` (`rubix/docs/design/
//! EXTENSION-RUNTIME.md`, phase 1: "restart with backoff"). Restart wait doubles
//! each step until [`Backoff::max_ms`], optionally with jitter to spread
//! restarts across replicas. The intensity cap (max N restarts within M seconds)
//! is a *separate* concern in [`RestartTracker`](super::restart::RestartTracker);
//! this module only computes the next sleep.
//!
//! Unlike starter, jitter here is computed from a tiny self-contained xorshift
//! seeded at construction, **not** the `rand` crate — the supervisor must not
//! drag a new workspace dependency in for a 0–50% spread, and a deterministic
//! seed keeps the schedule reproducible in tests.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// The backoff configuration carried on an extension's process spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backoff {
    /// First wait, in milliseconds (also the value [`BackoffSchedule::reset`]
    /// returns to).
    pub initial_ms: u32,
    /// Ceiling the doubling is capped at, in milliseconds.
    pub max_ms: u32,
    /// Whether to add up to 50% jitter on top of each wait.
    pub jitter: bool,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            initial_ms: 200,
            max_ms: 30_000,
            jitter: true,
        }
    }
}

/// Iterator-style schedule returning the next sleep. Doubles each step until
/// [`Backoff::max_ms`]; resets to `initial_ms` after a child is "stable" long
/// enough that prior crashes are considered paid for (the supervisor owns the
/// reset call site).
#[derive(Debug, Clone)]
pub struct BackoffSchedule {
    initial: Duration,
    max: Duration,
    jitter: bool,
    next: Duration,
    /// xorshift state for the 0–50% jitter; seeded from the config so a test
    /// schedule is reproducible without `rand`.
    rng: u64,
}

impl BackoffSchedule {
    /// Build a schedule from a [`Backoff`] block.
    #[must_use]
    pub fn from_config(cfg: &Backoff) -> Self {
        let initial = Duration::from_millis(u64::from(cfg.initial_ms.max(1)));
        let max = Duration::from_millis(u64::from(cfg.max_ms.max(cfg.initial_ms)));
        Self {
            initial,
            max,
            jitter: cfg.jitter,
            next: initial,
            // A non-zero seed derived from the config so identical configs walk
            // identical jitter sequences (deterministic, no global RNG).
            rng: u64::from(cfg.initial_ms)
                .wrapping_mul(2_654_435_761)
                .wrapping_add(0x9E37_79B9)
                | 1,
        }
    }

    /// Reset to the initial wait, after a child has been stable long enough.
    pub fn reset(&mut self) {
        self.next = self.initial;
    }

    /// Take the next wait and advance the schedule.
    pub fn next_wait(&mut self) -> Duration {
        let base = self.next.min(self.max);
        let following = base.saturating_mul(2).min(self.max);
        self.next = if following == Duration::ZERO {
            self.initial
        } else {
            following
        };

        if self.jitter {
            let extra_ms = (base.as_millis() as u64).saturating_div(2);
            if extra_ms > 0 {
                let jitter_ms = self.next_rand() % (extra_ms + 1);
                return base + Duration::from_millis(jitter_ms);
            }
        }
        base
    }

    /// xorshift64 step — a tiny, dependency-free PRNG for jitter spread.
    fn next_rand(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(initial: u32, max: u32, jitter: bool) -> Backoff {
        Backoff {
            initial_ms: initial,
            max_ms: max,
            jitter,
        }
    }

    #[test]
    fn doubles_until_cap() {
        let mut s = BackoffSchedule::from_config(&cfg(100, 1_000, false));
        assert_eq!(s.next_wait(), Duration::from_millis(100));
        assert_eq!(s.next_wait(), Duration::from_millis(200));
        assert_eq!(s.next_wait(), Duration::from_millis(400));
        assert_eq!(s.next_wait(), Duration::from_millis(800));
        assert_eq!(s.next_wait(), Duration::from_millis(1_000));
        assert_eq!(s.next_wait(), Duration::from_millis(1_000));
    }

    #[test]
    fn reset_returns_to_initial() {
        let mut s = BackoffSchedule::from_config(&cfg(50, 500, false));
        let _ = s.next_wait();
        let _ = s.next_wait();
        s.reset();
        assert_eq!(s.next_wait(), Duration::from_millis(50));
    }

    #[test]
    fn jitter_stays_within_bounds() {
        let mut s = BackoffSchedule::from_config(&cfg(100, 100, true));
        for _ in 0..20 {
            let w = s.next_wait();
            assert!(w >= Duration::from_millis(100));
            assert!(w <= Duration::from_millis(150));
        }
    }

    #[test]
    fn jitter_is_deterministic_for_a_given_config() {
        let mut a = BackoffSchedule::from_config(&cfg(100, 100, true));
        let mut b = BackoffSchedule::from_config(&cfg(100, 100, true));
        for _ in 0..10 {
            assert_eq!(a.next_wait(), b.next_wait());
        }
    }
}
