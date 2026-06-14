//! Exponential backoff with jitter for driver restart. Pure policy: given the
//! consecutive-failure count, yield the delay before the next spawn attempt.

use std::time::Duration;

/// Restart backoff policy. Delay doubles each consecutive failure up to a cap,
/// then full jitter is applied: the actual delay is uniform in `[0, computed]`,
/// which de-correlates a fleet of drivers restarting after a shared outage.
#[derive(Debug, Clone)]
pub struct Backoff {
    pub base: Duration,
    pub max: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(500),
            max: Duration::from_secs(30),
        }
    }
}

impl Backoff {
    /// Uncapped exponential delay for `failures` consecutive failures
    /// (`failures == 0` → `base`), saturating at `max`.
    pub fn ceiling(&self, failures: u32) -> Duration {
        let shift = failures.min(31);
        self.base
            .checked_mul(1u32 << shift)
            .unwrap_or(self.max)
            .min(self.max)
    }

    /// Apply full jitter to the ceiling using `jitter` in `[0.0, 1.0]`
    /// (injected so the policy stays deterministic under test).
    pub fn delay(&self, failures: u32, jitter: f64) -> Duration {
        let ceil = self.ceiling(failures).as_secs_f64();
        Duration::from_secs_f64(ceil * jitter.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceiling_doubles_then_caps() {
        let b = Backoff {
            base: Duration::from_millis(500),
            max: Duration::from_secs(30),
        };
        assert_eq!(b.ceiling(0), Duration::from_millis(500));
        assert_eq!(b.ceiling(1), Duration::from_secs(1));
        assert_eq!(b.ceiling(2), Duration::from_secs(2));
        // 500ms << 6 = 32s, capped to 30s.
        assert_eq!(b.ceiling(6), Duration::from_secs(30));
        assert_eq!(b.ceiling(1000), Duration::from_secs(30));
    }

    #[test]
    fn full_jitter_spans_zero_to_ceiling() {
        let b = Backoff::default();
        assert_eq!(b.delay(2, 0.0), Duration::ZERO);
        assert_eq!(b.delay(2, 1.0), b.ceiling(2));
        assert!(b.delay(2, 0.5) < b.ceiling(2));
    }
}
