//! Deterministic pseudo-random series for seeded history backfill.
//!
//! Mirrors the UI demo `series()` (a Park–Miller LCG over a sine carrier) so a
//! seeded numeric point's curve matches the look the operator approved. The
//! values are real rows once written through the store; the determinism keeps a
//! re-seed reproducible (constraint: idempotent seed).

/// Samples per day at the 30-minute cadence the UI ranges (24h/48h/7d) expect.
pub const SAMPLES_PER_DAY: usize = 48;
/// Backfill horizon: 7 days of 30-minute samples per numeric point.
pub const BACKFILL_DAYS: usize = 7;
/// Total samples a numeric point is backfilled with.
pub const BACKFILL_SAMPLES: usize = BACKFILL_DAYS * SAMPLES_PER_DAY;

/// A curve shape: a base level, a swing amplitude, and a seed selecting the
/// wobble sequence. Matches the UI fixture `Seed`.
#[derive(Debug, Clone, Copy)]
pub struct Curve {
    pub base: f64,
    pub amp: f64,
    pub seed: i64,
}

/// `n` samples of `base + sine(period) ± wobble`, rounded to 2 decimals.
/// `period` is the carrier wavelength in samples (default a full day).
pub fn series(curve: Curve, n: usize, period: usize) -> Vec<f64> {
    let mut s = curve.seed % 2_147_483_647;
    if s <= 0 {
        s += 2_147_483_646;
    }
    let mut rng = move || {
        s = (s * 16_807) % 2_147_483_647;
        s as f64 / 2_147_483_647.0
    };
    (0..n)
        .map(|i| {
            let wave = ((i as f64 / period as f64) * std::f64::consts::PI * 2.0).sin() * curve.amp;
            let wobble = (rng() - 0.5) * curve.amp * 0.5;
            ((curve.base + wave + wobble) * 100.0).round() / 100.0
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_is_deterministic_for_a_seed() {
        let c = Curve { base: 13.5, amp: 1.4, seed: 11 };
        let a = series(c, BACKFILL_SAMPLES, SAMPLES_PER_DAY);
        let b = series(c, BACKFILL_SAMPLES, SAMPLES_PER_DAY);
        assert_eq!(a, b);
        assert_eq!(a.len(), BACKFILL_SAMPLES);
    }

    #[test]
    fn distinct_seeds_diverge() {
        let p = SAMPLES_PER_DAY;
        let a = series(Curve { base: 10.0, amp: 5.0, seed: 7 }, 96, p);
        let b = series(Curve { base: 10.0, amp: 5.0, seed: 8 }, 96, p);
        assert_ne!(a, b);
    }

    #[test]
    fn values_track_the_base_level() {
        let c = Curve { base: 100.0, amp: 4.0, seed: 3 };
        let xs = series(c, SAMPLES_PER_DAY, SAMPLES_PER_DAY);
        // base ± (amp + half-amp wobble) bounds every sample.
        for v in xs {
            assert!((90.0..=110.0).contains(&v), "{v} out of band");
        }
    }
}
