//! Roll a numeric time-series up into per-bucket aggregates.
//!
//! The values that feed a rule decision are time-window rollups of a numeric
//! series (`rubix/STACK-DEISGN.md`, "Rhai owns the decision; DataFusion owns the
//! data"). Given `(timestamp, value)` samples and a [`Grain`], this groups
//! samples into epoch-aligned buckets (see [`super::window`]) and computes
//! `avg/min/max/sum/count/first/last` for each — `first`/`last` ordered by
//! timestamp within the bucket. The fold here is the pure aggregation core; the
//! scoped-session scan that produces the samples lives in [`super::series`], so
//! the math is unit-testable without a database.

use std::collections::BTreeMap;

use super::window::Grain;

/// One numeric sample at an instant (epoch microseconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// The sample's instant, epoch microseconds.
    pub at_micros: i64,
    /// The numeric value at that instant.
    pub value: f64,
}

/// The aggregates of one epoch-aligned bucket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BucketRollup {
    /// The bucket's epoch-aligned start (epoch microseconds).
    pub bucket_start: i64,
    /// Mean of the bucket's values.
    pub avg: f64,
    /// Smallest value in the bucket.
    pub min: f64,
    /// Largest value in the bucket.
    pub max: f64,
    /// Sum of the bucket's values.
    pub sum: f64,
    /// Number of samples in the bucket.
    pub count: u64,
    /// The earliest sample's value (by timestamp) in the bucket.
    pub first: f64,
    /// The latest sample's value (by timestamp) in the bucket.
    pub last: f64,
}

/// Per-bucket accumulator carrying the ordering needed for first/last.
struct Accumulator {
    min: f64,
    max: f64,
    sum: f64,
    count: u64,
    first_at: i64,
    first: f64,
    last_at: i64,
    last: f64,
}

impl Accumulator {
    fn new(sample: Sample) -> Self {
        Self {
            min: sample.value,
            max: sample.value,
            sum: sample.value,
            count: 1,
            first_at: sample.at_micros,
            first: sample.value,
            last_at: sample.at_micros,
            last: sample.value,
        }
    }

    fn absorb(&mut self, sample: Sample) {
        self.min = self.min.min(sample.value);
        self.max = self.max.max(sample.value);
        self.sum += sample.value;
        self.count += 1;
        if sample.at_micros < self.first_at {
            self.first_at = sample.at_micros;
            self.first = sample.value;
        }
        if sample.at_micros >= self.last_at {
            self.last_at = sample.at_micros;
            self.last = sample.value;
        }
    }

    fn finish(&self, bucket_start: i64) -> BucketRollup {
        BucketRollup {
            bucket_start,
            avg: self.sum / self.count as f64,
            min: self.min,
            max: self.max,
            sum: self.sum,
            count: self.count,
            first: self.first,
            last: self.last,
        }
    }
}

/// Roll `samples` up into per-bucket aggregates at the given `grain`.
///
/// Buckets are epoch-aligned and returned in ascending bucket-start order (the
/// `BTreeMap` keeps the order stable for a rule comparing consecutive windows).
/// An empty input yields an empty result. Samples need not be pre-sorted —
/// `first`/`last` are resolved by timestamp inside each bucket regardless of
/// input order.
#[must_use]
pub fn rollup(samples: &[Sample], grain: Grain) -> Vec<BucketRollup> {
    let mut buckets: BTreeMap<i64, Accumulator> = BTreeMap::new();
    for &sample in samples {
        let start = grain.bucket_start(sample.at_micros);
        buckets
            .entry(start)
            .and_modify(|acc| acc.absorb(sample))
            .or_insert_with(|| Accumulator::new(sample));
    }
    buckets
        .into_iter()
        .map(|(start, acc)| acc.finish(start))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Sample, rollup};
    use crate::aggregate::window::Grain;

    fn sample(at_secs: i64, value: f64) -> Sample {
        Sample {
            at_micros: at_secs * 1_000_000,
            value,
        }
    }

    #[test]
    fn empty_input_yields_no_buckets() {
        assert!(rollup(&[], Grain::Minute).is_empty());
    }

    #[test]
    fn a_single_minute_bucket_aggregates_every_function() {
        let samples = [sample(0, 10.0), sample(20, 20.0), sample(40, 30.0)];
        let out = rollup(&samples, Grain::Minute);
        assert_eq!(out.len(), 1);
        let b = out[0];
        assert_eq!(b.bucket_start, 0);
        assert!((b.avg - 20.0).abs() < f64::EPSILON);
        assert_eq!(b.min, 10.0);
        assert_eq!(b.max, 30.0);
        assert_eq!(b.sum, 60.0);
        assert_eq!(b.count, 3);
        assert_eq!(b.first, 10.0);
        assert_eq!(b.last, 30.0);
    }

    #[test]
    fn samples_split_across_minute_boundaries() {
        // 30s and 90s land in different minute buckets.
        let samples = [sample(30, 1.0), sample(90, 2.0)];
        let out = rollup(&samples, Grain::Minute);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].bucket_start, 0);
        assert_eq!(out[1].bucket_start, Grain::Minute.width_micros());
    }

    #[test]
    fn first_and_last_follow_timestamp_not_input_order() {
        // Out-of-order input within one bucket.
        let samples = [sample(40, 4.0), sample(10, 1.0), sample(25, 2.5)];
        let out = rollup(&samples, Grain::Minute);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].first, 1.0);
        assert_eq!(out[0].last, 4.0);
    }

    #[test]
    fn buckets_are_returned_in_ascending_order() {
        let samples = [sample(7200, 3.0), sample(0, 1.0), sample(3600, 2.0)];
        let out = rollup(&samples, Grain::Hour);
        let starts: Vec<i64> = out.iter().map(|b| b.bucket_start).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
    }
}
