//! Decimate a sample stream down to a target rate.
//!
//! High-rate streams are reduced *in flight*, before persistence, so the store
//! is not written at the raw sensor rate (`rubix/docs/SCOPE.md`, "Ingestion and
//! pre-processing": raw high-rate streams are processed before persistence, not
//! written first and queried back). Decimation keeps one sample out of every
//! `factor` and drops the rest — the simplest rate reduction that preserves the
//! stream's phase (the first sample of each window is the one kept), with no
//! interpolation that would invent values the source never sent.

use crate::subscribe::Sample;

/// A stateful 1-in-`factor` decimator over a sample stream.
///
/// The counter advances per [`admit`](Decimator::admit) call, so the same
/// decimator threaded through a stream keeps every `factor`-th sample. A `factor`
/// of 1 keeps everything (a no-op decimator); the counter wraps so the cadence is
/// stable for an unbounded stream.
#[derive(Debug, Clone)]
pub struct Decimator {
    factor: u32,
    seen: u32,
}

impl Decimator {
    /// Build a decimator keeping one sample out of every `factor`.
    ///
    /// A `factor` of 0 is meaningless (it would keep nothing); it is clamped to 1
    /// so the decimator never silently swallows an entire stream.
    #[must_use]
    pub fn new(factor: u32) -> Self {
        Self {
            factor: factor.max(1),
            seen: 0,
        }
    }

    /// Decide whether to keep `sample`, advancing the decimation phase.
    ///
    /// Returns `Some(sample)` for every `factor`-th sample and `None` for the
    /// ones dropped. The sample is moved through unchanged when kept — decimation
    /// drops, it never rewrites content.
    #[must_use]
    pub fn admit(&mut self, sample: Sample) -> Option<Sample> {
        let keep = self.seen.is_multiple_of(self.factor);
        self.seen = (self.seen + 1) % self.factor;
        if keep { Some(sample) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::Decimator;
    use crate::subscribe::Sample;

    fn sample(n: i64) -> Sample {
        Sample::new("rubix/ingest/edge/temp", serde_json::json!({ "n": n }))
    }

    #[test]
    fn factor_one_keeps_every_sample() {
        let mut decimator = Decimator::new(1);
        let kept: Vec<_> = (0..5).filter_map(|n| decimator.admit(sample(n))).collect();
        assert_eq!(kept.len(), 5);
    }

    #[test]
    fn factor_three_keeps_one_in_three() {
        let mut decimator = Decimator::new(3);
        let kept: Vec<_> = (0..9).filter_map(|n| decimator.admit(sample(n))).collect();
        assert_eq!(kept.len(), 3);
        assert_eq!(kept[0].content, serde_json::json!({ "n": 0 }));
        assert_eq!(kept[1].content, serde_json::json!({ "n": 3 }));
        assert_eq!(kept[2].content, serde_json::json!({ "n": 6 }));
    }

    #[test]
    fn factor_zero_is_clamped_and_never_swallows_the_stream() {
        let mut decimator = Decimator::new(0);
        let kept: Vec<_> = (0..4).filter_map(|n| decimator.admit(sample(n))).collect();
        assert_eq!(kept.len(), 4);
    }
}
