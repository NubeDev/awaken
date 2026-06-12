//! Write the blueprint through the store layer (never raw SQL) so every
//! invariant holds: priority-array validation on `command_point`, history on
//! effective-value change, sensor ingest on `ingest_cur`.
//!
//! Idempotent by slug: a site/equip/point/spark that already exists is left
//! untouched, so re-seeding a populated store neither duplicates nor errors
//! (constraint: idempotent dev seed).

use std::collections::HashMap;

use chrono::{Duration, Utc};
use rubix_core::{Equip, Point, PointKind, PointValue, PriorityArray, Spark, SparkSeverity};
use uuid::Uuid;

use super::curves::{series, Curve, BACKFILL_SAMPLES, SAMPLES_PER_DAY};
use super::portfolio::{
    EquipSpec, PointKindSpec, PointSpec, SiteSpec, SparkSeveritySpec, SparkSpec, EQUIPS, ORG,
    POINTS, SITES, SPARKS,
};
use super::tags::markers;
use super::SeedError;
use crate::store::Store;

/// Outcome of a seed run: how many of each resource were newly created. Zero
/// across the board means an already-seeded store (idempotent no-op).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SeedReport {
    pub sites: usize,
    pub equips: usize,
    pub points: usize,
    pub his_samples: usize,
    pub sparks: usize,
}

/// Populate `store` with the demo portfolio. Idempotent: existing rows (matched
/// by slug/path) are skipped.
pub fn seed_portfolio(store: &Store) -> Result<SeedReport, SeedError> {
    let mut report = SeedReport::default();
    for (site_idx, site_spec) in SITES.iter().enumerate() {
        let site_id = ensure_site(store, site_spec, &mut report)?;
        let equip_ids = ensure_equips(store, site_id, &mut report)?;
        let point_ids = ensure_points(store, &equip_ids, site_idx, &mut report)?;
        ensure_sparks(store, site_id, &point_ids, &mut report)?;
    }
    Ok(report)
}

fn ensure_site(
    store: &Store,
    spec: &SiteSpec,
    report: &mut SeedReport,
) -> Result<Uuid, SeedError> {
    if let Some(existing) = store
        .list_sites(Some(ORG))?
        .into_iter()
        .find(|s| s.slug == spec.slug)
    {
        return Ok(existing.id);
    }
    let site = rubix_core::Site {
        id: Uuid::new_v4(),
        org: ORG.to_string(),
        slug: spec.slug.to_string(),
        display_name: spec.display_name.to_string(),
        tags: markers(spec.tags),
        created_at: Utc::now(),
    };
    let id = site.id;
    store.create_site(&site)?;
    report.sites += 1;
    Ok(id)
}

/// Create the equip blueprint under a site, returning `path -> equip id` for
/// the whole blueprint (existing equips included, so points always resolve).
fn ensure_equips(
    store: &Store,
    site_id: Uuid,
    report: &mut SeedReport,
) -> Result<HashMap<&'static str, Uuid>, SeedError> {
    let existing: HashMap<String, Uuid> = store
        .list_equips(Some(site_id), &[])?
        .into_iter()
        .map(|e| (e.path, e.id))
        .collect();
    let mut ids = HashMap::new();
    for spec in EQUIPS {
        if let Some(id) = existing.get(spec.path) {
            ids.insert(spec.path, *id);
            continue;
        }
        let equip = build_equip(site_id, spec);
        let id = equip.id;
        store.create_equip(&equip)?;
        report.equips += 1;
        ids.insert(spec.path, id);
    }
    Ok(ids)
}

fn build_equip(site_id: Uuid, spec: &EquipSpec) -> Equip {
    Equip {
        id: Uuid::new_v4(),
        site_id,
        path: spec.path.to_string(),
        display_name: spec.display_name.to_string(),
        tags: markers(spec.tags),
        created_at: Utc::now(),
    }
}

/// Create the point blueprint, populating priority slots and history through
/// the store command/ingest path. Returns `equip-slug -> point id` keyed the
/// way the spark blueprint references implicated points.
fn ensure_points(
    store: &Store,
    equip_ids: &HashMap<&'static str, Uuid>,
    site_idx: usize,
    report: &mut SeedReport,
) -> Result<HashMap<String, Uuid>, SeedError> {
    let mut point_ids = HashMap::new();
    for spec in POINTS {
        let equip_id = *equip_ids
            .get(spec.equip)
            .ok_or_else(|| SeedError::MissingEquip(spec.equip))?;
        let key = format!("{}-{}", spec.equip, spec.slug);
        if let Some(existing) = store
            .list_points(Some(equip_id), None, &[])?
            .into_iter()
            .find(|p| p.slug == spec.slug)
        {
            point_ids.insert(key, existing.id);
            continue;
        }
        let id = create_point(store, equip_id, spec)?;
        apply_slots(store, id, spec)?;
        report.his_samples += backfill_history(store, id, spec, site_idx)?;
        report.points += 1;
        point_ids.insert(key, id);
    }
    Ok(point_ids)
}

fn create_point(store: &Store, equip_id: Uuid, spec: &PointSpec) -> Result<Uuid, SeedError> {
    let mut priority_array = PriorityArray::new();
    priority_array.relinquish_default = spec.relinquish_default.map(PointValue::Number);
    // Sensors carry their cur directly; writable points derive it from slots.
    let (cur_value, cur_ts) = match (spec.cur_str, spec.cur_num) {
        (Some(s), _) => (Some(PointValue::Str(s.to_string())), Some(Utc::now())),
        (None, Some(n)) if matches!(spec.kind, PointKindSpec::Sensor) => {
            (Some(PointValue::Number(n)), Some(Utc::now()))
        }
        _ => {
            let eff = priority_array.effective().map(|(_, v)| v.clone());
            (eff.clone(), eff.map(|_| Utc::now()))
        }
    };
    let point = Point {
        id: Uuid::new_v4(),
        equip_id,
        slug: spec.slug.to_string(),
        display_name: spec.display_name.to_string(),
        kind: point_kind(spec.kind),
        unit: unit_or_none(spec.unit),
        tags: markers(spec.tags),
        priority_array,
        cur_value,
        cur_ts,
        created_at: Utc::now(),
    };
    let id = point.id;
    store.create_point(&point)?;
    Ok(id)
}

/// Command each blueprint slot through `command_point` so the priority-array
/// invariants are enforced and the effective value lands in history exactly as
/// a live operator/agent write would.
fn apply_slots(store: &Store, id: Uuid, spec: &PointSpec) -> Result<(), SeedError> {
    for (level, value) in spec.slots {
        store.command_point(id, *level, Some(PointValue::Number(*value)), Utc::now())?;
    }
    Ok(())
}

/// Backfill 7 days of 30-min samples for a numeric point through `his_insert`.
/// Sensors take their curve; writable points trend their effective command.
fn backfill_history(
    store: &Store,
    id: Uuid,
    spec: &PointSpec,
    site_idx: usize,
) -> Result<usize, SeedError> {
    let Some(curve) = spec.curve else {
        return Ok(0);
    };
    // Offset the seed per site so each site's curves differ (matches the UI).
    let curve = Curve {
        seed: curve.seed + (site_idx as i64) * 7,
        ..curve
    };
    let values = series(curve, BACKFILL_SAMPLES, SAMPLES_PER_DAY);
    let now = Utc::now();
    let samples: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, &value)| rubix_core::HisSample {
            // Sample 0 is the oldest (7 days ago); the last is ~now.
            ts: now - Duration::minutes(((BACKFILL_SAMPLES - i) as i64) * 30),
            value: PointValue::Number(value),
        })
        .collect();
    Ok(store.his_insert(id, &samples)?)
}

fn ensure_sparks(
    store: &Store,
    site_id: Uuid,
    point_ids: &HashMap<String, Uuid>,
    report: &mut SeedReport,
) -> Result<(), SeedError> {
    let existing: Vec<String> = store
        .list_sparks(Some(site_id), None, None)?
        .into_iter()
        .map(|s| s.rule)
        .collect();
    for spec in SPARKS {
        if existing.iter().any(|r| r == spec.rule) {
            continue;
        }
        let spark = build_spark(site_id, spec, point_ids)?;
        store.create_spark(&spark)?;
        report.sparks += 1;
    }
    Ok(())
}

fn build_spark(
    site_id: Uuid,
    spec: &SparkSpec,
    point_ids: &HashMap<String, Uuid>,
) -> Result<Spark, SeedError> {
    let mut implicated = Vec::with_capacity(spec.points.len());
    for key in spec.points {
        let id = point_ids
            .get(*key)
            .ok_or_else(|| SeedError::MissingPoint(key))?;
        implicated.push(*id);
    }
    Ok(Spark {
        id: Uuid::new_v4(),
        site_id,
        rule: spec.rule.to_string(),
        severity: spark_severity(spec.severity),
        message: spec.message.to_string(),
        point_ids: implicated,
        ts: Utc::now(),
        acknowledged: spec.acknowledged,
    })
}

fn point_kind(k: PointKindSpec) -> PointKind {
    match k {
        PointKindSpec::Sensor => PointKind::Sensor,
        PointKindSpec::Cmd => PointKind::Cmd,
        PointKindSpec::Sp => PointKind::Sp,
    }
}

fn spark_severity(s: SparkSeveritySpec) -> SparkSeverity {
    match s {
        SparkSeveritySpec::Info => SparkSeverity::Info,
        SparkSeveritySpec::Warning => SparkSeverity::Warning,
        SparkSeveritySpec::Fault => SparkSeverity::Fault,
    }
}

fn unit_or_none(unit: &str) -> Option<String> {
    if unit.is_empty() {
        None
    } else {
        Some(unit.to_string())
    }
}
