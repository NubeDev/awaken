//! Dev-only live `cur` ticker for seeded sensor points.
//!
//! The reference `rubix-driver-sim` publishes one configured point per process,
//! so driving all seeded sensors that way is disproportionate. Instead, when
//! the store is seeded in dev, a single detached task ingests a fresh sample for
//! every seeded numeric sensor on an interval through the real ingest path
//! (`Store::ingest_cur`, the same call `POST /points/{id}/cur` makes) and
//! publishes it on the bus when one is present — so the UI ticks live in dev
//! with zero fake data. Never started outside `--seed-dev`.

use std::time::Duration;

use chrono::Utc;
use rubix_core::{HisSample, PointKind, PointValue};
use tokio::sync::watch;
use uuid::Uuid;

use super::curves::{series, Curve, SAMPLES_PER_DAY};
use super::portfolio::POINTS;
use crate::bus::ZenohBus;
use crate::store::Store;

/// Default cadence for the dev `cur` ticker.
const TICK_PERIOD: Duration = Duration::from_secs(5);

/// A seeded sensor point and the curve that drives its live value.
struct TickTarget {
    id: Uuid,
    keyexpr: String,
    curve: Curve,
}

/// Handle to the running ticker; dropping the sender stops the loop.
pub struct DevTicker {
    stop: watch::Sender<bool>,
}

impl DevTicker {
    /// Signal the ticker loop to stop and let it drain.
    pub async fn shutdown(self) {
        let _ = self.stop.send(true);
    }
}

/// Spawn the dev ticker over the seeded numeric sensor points. Resolves each
/// blueprint sensor to its stored id/keyexpr; points absent from the store
/// (e.g. a partial seed) are skipped. Returns `None` when nothing is tickable.
pub fn spawn(store: Store, bus: Option<ZenohBus>) -> Option<DevTicker> {
    let targets = resolve_targets(&store);
    if targets.is_empty() {
        return None;
    }
    let (stop, mut stop_rx) = watch::channel(false);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(TICK_PERIOD);
        // The phase advances each tick so the value walks along the curve.
        let mut phase: usize = 0;
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    drive_once(&store, bus.as_ref(), &targets, phase).await;
                    phase = phase.wrapping_add(1);
                }
                _ = stop_rx.changed() => {
                    if *stop_rx.borrow() { break; }
                }
            }
        }
    });
    Some(DevTicker { stop })
}

fn resolve_targets(store: &Store) -> Vec<TickTarget> {
    let mut targets = Vec::new();
    let Ok(sites) = store.list_sites(Some(super::portfolio::ORG)) else {
        return targets;
    };
    for (site_idx, site) in sites_in_blueprint_order(store, &sites).into_iter().enumerate() {
        let Ok(points) = store.list_points(None, Some(site.id), &[]) else {
            continue;
        };
        for spec in POINTS {
            let Some(curve) = spec.curve else { continue };
            if !matches!(spec.kind, super::portfolio::PointKindSpec::Sensor) {
                continue;
            }
            let Some(point) = points
                .iter()
                .find(|p| p.slug == spec.slug && p.kind == PointKind::Sensor)
            else {
                continue;
            };
            let Ok(keyexpr) = store.point_keyexpr(point.id) else {
                continue;
            };
            targets.push(TickTarget {
                id: point.id,
                keyexpr,
                // Offset per site so each site's live walk differs (matches seed).
                curve: Curve { seed: curve.seed + (site_idx as i64) * 7, ..curve },
            });
        }
    }
    targets
}

/// Sites in the blueprint's declared order so the per-site curve seed offset
/// matches the one the history backfill used.
fn sites_in_blueprint_order(
    _store: &Store,
    sites: &[rubix_core::Site],
) -> Vec<rubix_core::Site> {
    super::portfolio::SITES
        .iter()
        .filter_map(|spec| sites.iter().find(|s| s.slug == spec.slug).cloned())
        .collect()
}

async fn drive_once(
    store: &Store,
    bus: Option<&ZenohBus>,
    targets: &[TickTarget],
    phase: usize,
) {
    for target in targets {
        // One fresh sample at the current phase along the point's day curve.
        let idx = phase % SAMPLES_PER_DAY;
        let value = series(target.curve, SAMPLES_PER_DAY, SAMPLES_PER_DAY)
            .get(idx)
            .copied()
            .unwrap_or(target.curve.base);
        let sample = HisSample {
            ts: Utc::now(),
            value: PointValue::Number(value),
        };
        let store = store.clone();
        let id = target.id;
        let ingested =
            tokio::task::spawn_blocking(move || store.ingest_cur(id, &sample)).await;
        match ingested {
            Ok(Ok(point)) => {
                if let Some(bus) = bus {
                    bus.publish_cur(&target.keyexpr, point.cur_value.as_ref()).await;
                }
            }
            Ok(Err(err)) => tracing::warn!(point = %target.id, %err, "dev ticker ingest failed"),
            Err(err) => tracing::warn!(point = %target.id, %err, "dev ticker join failed"),
        }
    }
}
