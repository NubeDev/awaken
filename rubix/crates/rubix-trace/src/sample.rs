//! The sampling decision — drop a fraction of spans before they persist.
//!
//! Traces are high volume and not kept forever, so a configurable fraction is
//! dropped before the durable write (`rubix/docs/SCOPE.md`, "Tracing"; contract
//! #4 in `rubix/STACK-DEISGN.md`). The fraction is the *drop* fraction read from
//! `RUBIX_TRACE_SAMPLE`: `0.0` persists every span, `1.0` drops every span,
//! `0.25` drops a quarter.
//!
//! The admit/drop decision is derived from the span id rather than a running RNG,
//! so it needs no shared mutable state and is deterministic per span — the same
//! span always decides the same way. Over a large population of unique ids the
//! kept fraction converges on `1.0 - drop` (verified in `tests/`).

use crate::span::Span;

/// The environment variable carrying the drop fraction.
const SAMPLE_ENV: &str = "RUBIX_TRACE_SAMPLE";

/// The resolution of the id-derived bucket: a span maps to one of `BUCKETS`
/// evenly spread positions, and is dropped when its position falls below the
/// drop threshold. A larger value tightens how closely the realized rate tracks
/// the configured fraction.
const BUCKETS: u64 = 10_000;

/// A clamped drop fraction in `[0.0, 1.0]`.
///
/// Construction clamps out-of-range and non-finite inputs to the nearest valid
/// bound so a misconfigured env var degrades to "keep everything" rather than
/// failing a traced operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleRate {
    drop_fraction: f64,
}

impl SampleRate {
    /// Build a sample rate from a raw drop fraction, clamping to `[0.0, 1.0]`.
    ///
    /// A non-finite input (`NaN`, infinity) clamps to `0.0` — keep everything —
    /// because dropping all traces on a parse glitch would silently blind the
    /// platform.
    #[must_use]
    pub fn new(drop_fraction: f64) -> Self {
        let clamped = if drop_fraction.is_finite() {
            drop_fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            drop_fraction: clamped,
        }
    }

    /// Resolve the sample rate from `RUBIX_TRACE_SAMPLE`.
    ///
    /// An unset or unparseable value defaults to `0.0` (keep everything), so
    /// tracing is on by default and only thinned when explicitly configured.
    #[must_use]
    pub fn from_env() -> Self {
        let raw = std::env::var(SAMPLE_ENV).ok();
        let parsed = raw.and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(0.0);
        Self::new(parsed)
    }

    /// The clamped drop fraction.
    #[must_use]
    pub fn drop_fraction(&self) -> f64 {
        self.drop_fraction
    }

    /// Whether `span` should be persisted (admitted) under this rate.
    ///
    /// Deterministic per span id: the id hashes into one of [`BUCKETS`] evenly
    /// spread positions, and the span is admitted when its position is at or
    /// above the drop threshold.
    #[must_use]
    pub fn admits(&self, span: &Span) -> bool {
        if self.drop_fraction <= 0.0 {
            return true;
        }
        if self.drop_fraction >= 1.0 {
            return false;
        }
        let position = bucket(span.span_id.as_str());
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let threshold = (self.drop_fraction * BUCKETS as f64) as u64;
        position >= threshold
    }
}

/// Map a span id to a bucket in `[0, BUCKETS)` with a stable hash.
///
/// FNV-1a over the id bytes gives a well-spread, dependency-free hash; the modulo
/// folds it into the bucket range. Stability matters more than cryptographic
/// quality here — the only requirement is an even spread of distinct ids.
fn bucket(id: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash % BUCKETS
}

#[cfg(test)]
mod tests {
    use rubix_core::CorrelationId;

    use crate::span::Span;

    use super::SampleRate;

    fn span() -> Span {
        Span::root(CorrelationId::mint(), "step", serde_json::json!({}), 0, 1)
    }

    #[test]
    fn zero_drop_admits_everything() {
        let rate = SampleRate::new(0.0);
        for _ in 0..1_000 {
            assert!(rate.admits(&span()));
        }
    }

    #[test]
    fn full_drop_admits_nothing() {
        let rate = SampleRate::new(1.0);
        for _ in 0..1_000 {
            assert!(!rate.admits(&span()));
        }
    }

    #[test]
    fn out_of_range_and_non_finite_clamp_to_keep_everything() {
        assert_eq!(SampleRate::new(-0.5).drop_fraction(), 0.0);
        assert_eq!(SampleRate::new(2.0).drop_fraction(), 1.0);
        assert_eq!(SampleRate::new(f64::NAN).drop_fraction(), 0.0);
        assert_eq!(SampleRate::new(f64::INFINITY).drop_fraction(), 0.0);
    }

    #[test]
    fn realized_drop_rate_tracks_the_configured_fraction() {
        let rate = SampleRate::new(0.25);
        let total = 20_000;
        let dropped = (0..total).filter(|_| !rate.admits(&span())).count();
        let realized = dropped as f64 / f64::from(total);
        assert!((realized - 0.25).abs() < 0.02, "realized drop {realized}");
    }

    #[test]
    fn the_decision_is_stable_for_one_span() {
        let rate = SampleRate::new(0.5);
        let s = span();
        let first = rate.admits(&s);
        for _ in 0..100 {
            assert_eq!(rate.admits(&s), first);
        }
    }
}
