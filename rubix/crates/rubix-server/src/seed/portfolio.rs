//! The demo building portfolio: two tenants, two sites each, three domains.
//!
//! Every node is a generic `record` (schemaless content) connected by the tag
//! graph (`record → tagged → tag`), so the seed exercises the same model the
//! platform ships — no special seed tables. The shape is Project-Haystack-ish:
//! `site → equip → point → reading`, with HVAC, energy, and water equipment per
//! site. All writes cross the WS-05 gate as the tenant operator, so the seeded
//! store carries real audit rows, undo history, and live-query events.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rubix_core::{Id, Principal, Tag, attach_tag, create_tag};
use rubix_gate::{Capability, Change, Command, apply};
use serde_json::{Value, json};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::SeedError;
use super::history::{Series, readings};

/// One measured point on a piece of equipment.
struct PointSpec {
    /// Stable key, unique within its equipment.
    key: &'static str,
    /// Human-facing label.
    name: &'static str,
    /// What it measures (`temp`, `co2`, `kw`, `flow`, ...).
    measure: &'static str,
    /// Engineering unit.
    unit: &'static str,
    /// Central value (or accumulation rate, if `cumulative`).
    base: f64,
    /// Peak deviation from `base` for an oscillating point.
    swing: f64,
    /// Whether the point accumulates (a meter total).
    cumulative: bool,
    /// Marker tags beyond the structural ones every point gets.
    tags: &'static [&'static str],
}

/// A piece of equipment carrying points, scoped to one domain.
struct EquipSpec {
    /// Stable key, unique within its site.
    key: &'static str,
    /// Human-facing label.
    name: &'static str,
    /// Domain band: `hvac` / `energy` / `water`.
    domain: &'static str,
    /// Equipment type marker (`ahu`, `vav`, `meter`).
    kind: &'static str,
    /// The points hung off this equipment.
    points: &'static [PointSpec],
}

const AHU_POINTS: &[PointSpec] = &[
    PointSpec {
        key: "zone-temp",
        name: "Zone Temperature",
        measure: "temp",
        unit: "degC",
        base: 22.0,
        swing: 2.0,
        cumulative: false,
        tags: &["temp", "zone"],
    },
    PointSpec {
        key: "supply-temp",
        name: "Supply Air Temperature",
        measure: "temp",
        unit: "degC",
        base: 15.0,
        swing: 3.0,
        cumulative: false,
        tags: &["temp", "discharge"],
    },
    PointSpec {
        key: "co2",
        name: "Zone CO₂",
        measure: "co2",
        unit: "ppm",
        base: 600.0,
        swing: 150.0,
        cumulative: false,
        tags: &["co2", "air"],
    },
    PointSpec {
        key: "damper",
        name: "Damper Position",
        measure: "damper",
        unit: "percent",
        base: 50.0,
        swing: 30.0,
        cumulative: false,
        tags: &["damper", "cmd"],
    },
    PointSpec {
        key: "setpoint",
        name: "Zone Temperature Setpoint",
        measure: "setpoint",
        unit: "degC",
        base: 22.0,
        swing: 0.5,
        cumulative: false,
        tags: &["sp", "temp"],
    },
];

const VAV_POINTS: &[PointSpec] = &[
    PointSpec {
        key: "zone-temp",
        name: "Zone Temperature",
        measure: "temp",
        unit: "degC",
        base: 23.0,
        swing: 2.5,
        cumulative: false,
        tags: &["temp", "zone"],
    },
    PointSpec {
        key: "damper",
        name: "Damper Position",
        measure: "damper",
        unit: "percent",
        base: 40.0,
        swing: 25.0,
        cumulative: false,
        tags: &["damper", "cmd"],
    },
];

const ELEC_POINTS: &[PointSpec] = &[
    PointSpec {
        key: "power",
        name: "Active Power",
        measure: "kw",
        unit: "kW",
        base: 120.0,
        swing: 40.0,
        cumulative: false,
        tags: &["power", "elec"],
    },
    PointSpec {
        key: "energy",
        name: "Imported Energy",
        measure: "kwh",
        unit: "kWh",
        base: 50000.0,
        swing: 110.0,
        cumulative: true,
        tags: &["energy", "elec"],
    },
    PointSpec {
        key: "voltage",
        name: "Line Voltage",
        measure: "voltage",
        unit: "V",
        base: 230.0,
        swing: 5.0,
        cumulative: false,
        tags: &["voltage", "elec"],
    },
];

const WATER_POINTS: &[PointSpec] = &[
    PointSpec {
        key: "flow",
        name: "Flow Rate",
        measure: "flow",
        unit: "L/min",
        base: 30.0,
        swing: 15.0,
        cumulative: false,
        tags: &["flow", "water"],
    },
    PointSpec {
        key: "total",
        name: "Total Volume",
        measure: "volume",
        unit: "L",
        base: 2000000.0,
        swing: 1800.0,
        cumulative: true,
        tags: &["volume", "water"],
    },
    PointSpec {
        key: "pressure",
        name: "Supply Pressure",
        measure: "pressure",
        unit: "bar",
        base: 3.0,
        swing: 0.5,
        cumulative: false,
        tags: &["pressure", "water"],
    },
];

/// The equipment template instantiated identically at every site.
const EQUIPMENT: &[EquipSpec] = &[
    EquipSpec {
        key: "ahu-1",
        name: "Air Handling Unit 1",
        domain: "hvac",
        kind: "ahu",
        points: AHU_POINTS,
    },
    EquipSpec {
        key: "vav-1",
        name: "VAV Box 1",
        domain: "hvac",
        kind: "vav",
        points: VAV_POINTS,
    },
    EquipSpec {
        key: "elec-main",
        name: "Main Electricity Meter",
        domain: "energy",
        kind: "meter",
        points: ELEC_POINTS,
    },
    EquipSpec {
        key: "water-main",
        name: "Main Water Meter",
        domain: "water",
        kind: "meter",
        points: WATER_POINTS,
    },
];

/// The tenants and their sites: two tenants, two sites each.
const TENANTS: &[(&str, &[(&str, &str)])] = &[
    ("acme", &[("hq", "Acme HQ"), ("plant", "Acme Plant")]),
    (
        "globex",
        &[("tower", "Globex Tower"), ("campus", "Globex Campus")],
    ),
];

/// What one tenant's portfolio added, for the run summary.
pub struct TenantTally {
    /// The tenant namespace.
    pub namespace: &'static str,
    /// Number of site records written.
    pub sites: usize,
    /// Number of equipment + point records written.
    pub nodes: usize,
    /// Number of reading records written.
    pub readings: usize,
    /// Number of demo rule records written (filled by the rule seed).
    pub rules: usize,
}

/// Write the full portfolio for one tenant as `operator`, returning the tally.
pub async fn seed_tenant(
    db: &Surreal<Db>,
    namespace: &'static str,
    operator: &Principal,
    now: DateTime<Utc>,
    tags: &mut HashSet<String>,
) -> Result<TenantTally, SeedError> {
    let sites = TENANTS
        .iter()
        .find(|(ns, _)| *ns == namespace)
        .map(|(_, sites)| *sites)
        .unwrap_or(&[]);

    let mut tally = TenantTally {
        namespace,
        sites: 0,
        nodes: 0,
        readings: 0,
        rules: 0,
    };

    for (site_key, site_name) in sites {
        let site_id = id(&[namespace, site_key]);
        put(
            db,
            operator,
            &site_id,
            json!({
                "kind": "site", "key": site_key, "name": site_name,
            }),
        )
        .await?;
        attach(db, &site_id, &["site"], tags).await?;
        tally.sites += 1;

        for equip in EQUIPMENT {
            let equip_id = id(&[namespace, site_key, equip.key]);
            put(
                db,
                operator,
                &equip_id,
                json!({
                    "kind": "equip", "key": equip.key, "name": equip.name,
                    "domain": equip.domain, "type": equip.kind, "site": site_key,
                }),
            )
            .await?;
            attach(db, &equip_id, &["equip", equip.domain, equip.kind], tags).await?;
            tally.nodes += 1;

            for point in equip.points {
                let point_id = id(&[namespace, site_key, equip.key, point.key]);
                put(
                    db,
                    operator,
                    &point_id,
                    json!({
                        "kind": "point", "key": point.key, "name": point.name,
                        "domain": equip.domain, "measure": point.measure, "unit": point.unit,
                        "equip": equip.key, "site": site_key,
                    }),
                )
                .await?;
                let mut point_tags = vec!["point", "sensor", equip.domain];
                point_tags.extend_from_slice(point.tags);
                attach(db, &point_id, &point_tags, tags).await?;
                tally.nodes += 1;

                let series = Series {
                    point_id: &point_id,
                    measure: point.measure,
                    unit: point.unit,
                    domain: equip.domain,
                    site: site_key,
                    base: point.base,
                    swing: point.swing,
                    cumulative: point.cumulative,
                };
                for (reading_id, content) in readings(&series, now) {
                    put(db, operator, &reading_id, content).await?;
                    attach(db, &reading_id, &["reading", equip.domain], tags).await?;
                    tally.readings += 1;
                }
            }
        }
    }

    Ok(tally)
}

/// A deterministic, namespace-prefixed record id from its path segments.
///
/// Prefixing with the tenant keeps two tenants' identically-shaped trees from
/// colliding in the single shared `record` table.
fn id(segments: &[&str]) -> Id {
    Id::from_raw(segments.join("--"))
}

/// Create `content` at `target` through the gate as the tenant operator.
async fn put(
    db: &Surreal<Db>,
    operator: &Principal,
    target: &Id,
    content: Value,
) -> Result<(), SeedError> {
    let command = Command::new(
        operator.clone(),
        Capability::IngestPublish,
        target.clone(),
        Change::Create(content),
    );
    apply(db, &command, None)
        .await
        .map(|_| ())
        .map_err(|e| SeedError::new("write record", e))
}

/// Attach each named tag to `record`, creating the tag node once per run.
///
/// Tag ids are the tag name itself so a name maps to one node shared across
/// every record; `attach_tag` is idempotent, and the `seen` set keeps the
/// create call to once per name (a fresh store is assumed).
async fn attach(
    db: &Surreal<Db>,
    record: &Id,
    names: &[&str],
    seen: &mut HashSet<String>,
) -> Result<(), SeedError> {
    for name in names {
        if seen.insert((*name).to_owned()) {
            let tag = Tag {
                id: Id::from_raw(*name),
                name: (*name).to_owned(),
            };
            // Ignore an already-exists error so re-seeding a non-fresh store
            // still wires the edge below.
            let _ = create_tag(db, &tag).await;
        }
        attach_tag(db, record, &Id::from_raw(*name))
            .await
            .map_err(|e| SeedError::new("attach tag", e))?;
    }
    Ok(())
}
