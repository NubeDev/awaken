//! Synthetic time-series history for each seeded point.
//!
//! Topology records (sites, equipment, points) describe the plant; the readings
//! generated here give the query surface something to roll up. Each point gets
//! one sample per hour over the trailing window as a [`Reading`] in the data
//! plane — `at` carries the measurement instant (the query layer buckets on it,
//! never on receive-time `created`), so the trailing spread is real history.
//! Values are deterministic — a fixed wave, not randomness — so a seeded store is
//! reproducible and a diff of two runs is empty. Display metadata (`unit`,
//! `measure`, …) lives on the point record the `series` id points at, not copied
//! onto every lean sample.

use chrono::{DateTime, Duration, Utc};
use rubix_core::{Id, Reading};
use serde_json::json;

/// Hours of trailing history generated per point (one sample per hour).
const HISTORY_HOURS: i64 = 24;

/// A point's reading shape, enough to synthesize its trailing samples.
pub struct Series<'a> {
    /// The owning point's record id — the `series` every sample keys off.
    pub point_id: &'a Id,
    /// The central value the wave oscillates around (or the rate, if cumulative).
    pub base: f64,
    /// The peak deviation from `base` for an oscillating point.
    pub swing: f64,
    /// Whether the point accumulates (a meter total) rather than oscillates.
    pub cumulative: bool,
}

/// Build the trailing-window [`Reading`]s for one point in `namespace`, oldest
/// first, ending at `now`.
///
/// The `series` is the bare point id; content stays lean (`{}`) — the point
/// record carries the display metadata. Ids are derived from `(series, at)`, so a
/// re-seed re-appends the same rows idempotently.
pub fn readings(series: &Series, namespace: &str, now: DateTime<Utc>) -> Vec<Reading> {
    (0..HISTORY_HOURS)
        .map(|i| {
            let ts = now - Duration::hours(HISTORY_HOURS - 1 - i);
            let value = sample(series, i);
            Reading::new(
                namespace,
                series.point_id.as_str(),
                ts.into(),
                value,
                json!({}),
            )
        })
        .collect()
}

/// The value of the `i`-th sample: a rising ramp for a meter total, otherwise a
/// deterministic oscillation within `base ± swing`.
fn sample(series: &Series, i: i64) -> f64 {
    let raw = if series.cumulative {
        series.base + series.swing * i as f64
    } else {
        let frac = ((i * 37) % 100) as f64 / 100.0;
        series.base + series.swing * (frac * 2.0 - 1.0)
    };
    (raw * 100.0).round() / 100.0
}
