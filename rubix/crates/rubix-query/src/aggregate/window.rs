//! Epoch-aligned time-window bucketing.
//!
//! Rule decisions are fed by time-window rollups (`rubix/docs/SCOPE.md`,
//! "DataFusion — query and compute"; `rubix/STACK-DEISGN.md`, "Rhai owns the
//! decision; DataFusion owns the data"). A [`Grain`] is a fixed bucket width from
//! a minute to a week; bucketing is **epoch-aligned** — a timestamp's bucket is
//! the largest multiple of the grain at or before it, measured from the Unix
//! epoch — so the same instant always lands in the same bucket regardless of when
//! the query runs. Aligning to the epoch (not to "now" or to the first sample)
//! makes buckets stable and comparable across queries and across edges.

/// A fixed time-window width for rollup bucketing.
///
/// The grains the surface supports, smallest to largest. Each maps to a whole
/// number of microseconds, the resolution the Arrow timestamp column uses
/// (see [`super::super::provider`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    /// One-minute buckets.
    Minute,
    /// One-hour buckets.
    Hour,
    /// One-day buckets.
    Day,
    /// One-week buckets (aligned to the epoch, which was a Thursday).
    Week,
}

/// Microseconds in one second.
const MICROS_PER_SEC: i64 = 1_000_000;

impl Grain {
    /// Every grain, smallest to largest.
    pub const ALL: [Grain; 4] = [Grain::Minute, Grain::Hour, Grain::Day, Grain::Week];

    /// The stable wire/storage string for this grain.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Grain::Minute => "minute",
            Grain::Hour => "hour",
            Grain::Day => "day",
            Grain::Week => "week",
        }
    }

    /// Resolve a wire string to a grain, or `None` if unknown.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Grain> {
        Grain::ALL.into_iter().find(|grain| grain.as_str() == raw)
    }

    /// This grain's width in microseconds.
    #[must_use]
    pub fn width_micros(self) -> i64 {
        let seconds = match self {
            Grain::Minute => 60,
            Grain::Hour => 60 * 60,
            Grain::Day => 24 * 60 * 60,
            Grain::Week => 7 * 24 * 60 * 60,
        };
        seconds * MICROS_PER_SEC
    }

    /// The epoch-aligned bucket start (in epoch micros) for `micros`.
    ///
    /// The bucket is the largest multiple of [`Grain::width_micros`] at or before
    /// `micros`. Uses floored (not truncated) division so negative timestamps —
    /// instants before the Unix epoch — still floor toward the earlier bucket
    /// boundary rather than toward zero.
    #[must_use]
    pub fn bucket_start(self, micros: i64) -> i64 {
        let width = self.width_micros();
        let floored = micros.div_euclid(width);
        floored * width
    }

    /// Choose the grain whose buckets best approximate `target` points across a
    /// `span_micros`-wide window — the backend's single source of interval snap.
    ///
    /// The client sends a window plus a desired point count (§5, "backend owns the
    /// snap"); this picks the supported [`Grain`] that yields a bucket count
    /// closest to `target`, so the chart never recomputes the grain table itself.
    /// A non-positive span or target falls back to the finest grain. Ties prefer
    /// the finer grain (more detail), and the result is clamped to the supported
    /// set — never finer than a minute nor coarser than a week.
    #[must_use]
    pub fn for_target_points(span_micros: i64, target: u32) -> Grain {
        if span_micros <= 0 || target == 0 {
            return Grain::Minute;
        }
        // The ideal bucket width is the window divided into `target` buckets; pick
        // the supported grain whose width is nearest that ideal in log space, so a
        // 2× over and a 2× under are weighed evenly.
        let ideal = span_micros as f64 / f64::from(target);
        Grain::ALL
            .into_iter()
            .min_by(|a, b| {
                let da = (a.width_micros() as f64 / ideal).ln().abs();
                let db = (b.width_micros() as f64 / ideal).ln().abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(Grain::Minute)
    }
}

#[cfg(test)]
mod tests {
    use super::Grain;

    #[test]
    fn every_grain_round_trips_through_its_string() {
        for grain in Grain::ALL {
            assert_eq!(Grain::parse(grain.as_str()), Some(grain));
        }
    }

    #[test]
    fn an_unknown_grain_is_none() {
        assert_eq!(Grain::parse("fortnight"), None);
    }

    #[test]
    fn epoch_start_buckets_to_zero_for_every_grain() {
        for grain in Grain::ALL {
            assert_eq!(grain.bucket_start(0), 0);
        }
    }

    #[test]
    fn minute_aligns_to_the_minute_boundary() {
        let width = Grain::Minute.width_micros();
        // 90 seconds past the epoch falls in the [60s, 120s) minute bucket.
        let t = 90 * 1_000_000;
        assert_eq!(Grain::Minute.bucket_start(t), width);
    }

    #[test]
    fn a_timestamp_on_the_boundary_starts_its_own_bucket() {
        let width = Grain::Hour.width_micros();
        assert_eq!(Grain::Hour.bucket_start(width), width);
        assert_eq!(Grain::Hour.bucket_start(width - 1), 0);
    }

    #[test]
    fn day_and_week_align_from_the_epoch() {
        let day = Grain::Day.width_micros();
        // 25 hours past the epoch is in the second day bucket.
        let t = 25 * 60 * 60 * 1_000_000;
        assert_eq!(Grain::Day.bucket_start(t), day);

        let week = Grain::Week.width_micros();
        // 8 days past the epoch is in the second week bucket.
        let t = 8 * 24 * 60 * 60 * 1_000_000_i64;
        assert_eq!(Grain::Week.bucket_start(t), week);
    }

    #[test]
    fn target_points_snaps_to_the_nearest_grain() {
        let hour = Grain::Hour.width_micros();
        // A 24-hour window asking for ~24 points wants hour buckets.
        assert_eq!(Grain::for_target_points(24 * hour, 24), Grain::Hour);
        // The same window asking for ~1 point wants day buckets.
        assert_eq!(Grain::for_target_points(24 * hour, 1), Grain::Day);
        // A one-hour window asking for ~60 points wants minute buckets.
        assert_eq!(Grain::for_target_points(hour, 60), Grain::Minute);
        // A 90-day window asking for ~90 points wants day buckets.
        assert_eq!(Grain::for_target_points(90 * 24 * hour, 90), Grain::Day);
    }

    #[test]
    fn degenerate_target_inputs_fall_back_to_minute() {
        assert_eq!(Grain::for_target_points(0, 100), Grain::Minute);
        assert_eq!(Grain::for_target_points(60_000_000, 0), Grain::Minute);
    }

    #[test]
    fn pre_epoch_timestamps_floor_to_the_earlier_boundary() {
        let minute = Grain::Minute.width_micros();
        // One microsecond before the epoch is in the [-60s, 0s) bucket.
        assert_eq!(Grain::Minute.bucket_start(-1), -minute);
    }
}
