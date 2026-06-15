//! Synthetic time-series history for each seeded point.
//!
//! Topology records (sites, equipment, points) describe the plant; the readings
//! generated here give the query surface something to roll up. Each point gets
//! one sample per hour over the trailing window, carrying its own `ts` in the
//! content (the gate stamps every row `created = now`, so the spread must live in
//! the payload for `GROUP BY` time-bucket queries to mean anything). Values are
//! deterministic — a fixed wave, not randomness — so a seeded store is
//! reproducible and a diff of two runs is empty.

use chrono::{DateTime, Duration, Utc};
use rubix_core::Id;
use serde_json::{Value, json};

/// Hours of trailing history generated per point (one sample per hour).
const HISTORY_HOURS: i64 = 24;

/// A point's reading shape, enough to synthesize its trailing samples.
pub struct Series<'a> {
    /// The owning point's record id (readings key off it).
    pub point_id: &'a Id,
    /// What the point measures, e.g. `temp` / `kw` / `flow`.
    pub measure: &'a str,
    /// The engineering unit, e.g. `degC` / `kW` / `L/min`.
    pub unit: &'a str,
    /// The point's domain (`hvac` / `energy` / `water`).
    pub domain: &'a str,
    /// The owning site key.
    pub site: &'a str,
    /// The central value the wave oscillates around (or the rate, if cumulative).
    pub base: f64,
    /// The peak deviation from `base` for an oscillating point.
    pub swing: f64,
    /// Whether the point accumulates (a meter total) rather than oscillates.
    pub cumulative: bool,
}

/// Build the trailing-window reading records for one point as `(id, content)`
/// pairs, oldest first, ending at `now`.
pub fn readings(series: &Series, now: DateTime<Utc>) -> Vec<(Id, Value)> {
    (0..HISTORY_HOURS)
        .map(|i| {
            let ts = now - Duration::hours(HISTORY_HOURS - 1 - i);
            let value = sample(series, i);
            let id = Id::from_raw(format!("{}--r{i}", series.point_id.as_str()));
            let content = json!({
                "kind": "reading",
                "point": series.point_id.as_str(),
                "measure": series.measure,
                "domain": series.domain,
                "site": series.site,
                "ts": ts.to_rfc3339(),
                "value": value,
                "unit": series.unit,
            });
            (id, content)
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
