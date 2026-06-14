//! The demo portfolio blueprint: sites, the per-site equipment, the per-equip
//! points (with command-slot overrides and curve shapes), and the per-site
//! sparks. Ported from the UI demo fixtures (`ui/src/api/demo/fixtures.ts`),
//! which UI-03 deletes once the seed is the populated source of truth.
//!
//! Tags are load-bearing: the dashboard derives Load Breakdown from `submeter`
//! tags and KPIs from `meter`/`comfort` tags, so the tag tokens here must match
//! the UI's tag reads.

use super::curves::Curve;

/// A site definition. `org`/`slug` are keyexpr path segments.
pub struct SiteSpec {
    pub slug: &'static str,
    pub display_name: &'static str,
    pub tags: &'static [&'static str],
}

/// An equip under a site. `path` is the equip keyexpr segment.
pub struct EquipSpec {
    pub path: &'static str,
    pub display_name: &'static str,
    pub tags: &'static [&'static str],
}

/// A priority-slot override: `(level, value)`. Seeded through `command_point`
/// so the priority-array invariants hold and the effective value lands in
/// history exactly as a live write would.
pub type SlotOverride = (u8, f64);

/// A point under an equip. `equip` references an [`EquipSpec::path`].
pub struct PointSpec {
    pub equip: &'static str,
    pub slug: &'static str,
    pub display_name: &'static str,
    pub kind: PointKindSpec,
    pub unit: &'static str,
    pub tags: &'static [&'static str],
    /// Sensor/setpoint string cur (occupancy etc.); `None` for numeric points.
    pub cur_str: Option<&'static str>,
    /// Numeric cur for sensors without a curve (set directly via ingest).
    pub cur_num: Option<f64>,
    /// Command-slot overrides (writable points): `(level, value)`.
    pub slots: &'static [SlotOverride],
    /// Relinquish default for writable points.
    pub relinquish_default: Option<f64>,
    /// Curve shape for numeric history backfill; `None` skips backfill.
    pub curve: Option<Curve>,
}

#[derive(Clone, Copy)]
pub enum PointKindSpec {
    Sensor,
    Cmd,
    Sp,
}

/// A per-site spark finding. `points` references point keys `equip-slug`.
pub struct SparkSpec {
    pub rule: &'static str,
    pub severity: SparkSeveritySpec,
    pub message: &'static str,
    pub points: &'static [&'static str],
    pub acknowledged: bool,
}

#[derive(Clone, Copy)]
pub enum SparkSeveritySpec {
    Info,
    Warning,
    Fault,
}

pub const ORG: &str = "acme";

pub const SITES: &[SiteSpec] = &[
    SiteSpec {
        slug: "hq-tower",
        display_name: "HQ Tower",
        tags: &["site", "commercial"],
    },
    SiteSpec {
        slug: "distribution-w",
        display_name: "Distribution West",
        tags: &["site", "warehouse"],
    },
    SiteSpec {
        slug: "lab-campus",
        display_name: "Lab Campus",
        tags: &["site", "lab"],
    },
    SiteSpec {
        slug: "cold-store-3",
        display_name: "Cold Store 3",
        tags: &["site", "cold"],
    },
];

pub const EQUIPS: &[EquipSpec] = &[
    EquipSpec {
        path: "ahu-1",
        display_name: "AHU-1 · L1 East",
        tags: &["ahu", "hvac"],
    },
    EquipSpec {
        path: "ahu-3",
        display_name: "AHU-3 · L4 West",
        tags: &["ahu", "hvac"],
    },
    EquipSpec {
        path: "chiller-1",
        display_name: "Chiller-1",
        tags: &["chiller", "plant"],
    },
    EquipSpec {
        path: "chiller-2",
        display_name: "Chiller-2",
        tags: &["chiller", "plant"],
    },
    EquipSpec {
        path: "boiler-1",
        display_name: "Boiler-1",
        tags: &["boiler", "plant"],
    },
    EquipSpec {
        path: "meter-main",
        display_name: "Main Incomer",
        tags: &["elec", "meter", "energy"],
    },
    EquipSpec {
        path: "vav-4-12",
        display_name: "VAV 4-12",
        tags: &["vav", "hvac"],
    },
    EquipSpec {
        path: "vav-4-13",
        display_name: "VAV 4-13",
        tags: &["vav", "hvac"],
    },
    EquipSpec {
        path: "ct-1",
        display_name: "Cooling Tower 1",
        tags: &["tower", "plant"],
    },
];

const fn sensor(
    equip: &'static str,
    slug: &'static str,
    display_name: &'static str,
    unit: &'static str,
    tags: &'static [&'static str],
    cur_num: f64,
    curve: Curve,
) -> PointSpec {
    PointSpec {
        equip,
        slug,
        display_name,
        kind: PointKindSpec::Sensor,
        unit,
        tags,
        cur_str: None,
        cur_num: Some(cur_num),
        slots: &[],
        relinquish_default: None,
        curve: Some(curve),
    }
}

/// Point blueprint — AHU-3 carries the showcase command points; the main
/// incomer carries the demand meter + per-system submeters; zone sensors carry
/// the comfort index. Slot 13 is the agent ceiling, slot 16 the schedule.
pub const POINTS: &[PointSpec] = &[
    sensor(
        "ahu-3",
        "discharge-temp",
        "Discharge Air Temp",
        "°C",
        &["discharge", "air", "temp", "sensor"],
        13.4,
        Curve {
            base: 13.5,
            amp: 1.4,
            seed: 11,
        },
    ),
    sensor(
        "ahu-3",
        "return-temp",
        "Return Air Temp",
        "°C",
        &["return", "air", "temp", "sensor"],
        22.8,
        Curve {
            base: 22.6,
            amp: 0.8,
            seed: 21,
        },
    ),
    PointSpec {
        equip: "ahu-3",
        slug: "supply-fan-cmd",
        display_name: "Supply Fan Speed",
        kind: PointKindSpec::Cmd,
        unit: "%",
        tags: &["supply", "fan", "cmd"],
        cur_str: None,
        cur_num: None,
        slots: &[(8, 82.0), (13, 70.0), (16, 60.0)],
        relinquish_default: None,
        curve: Some(Curve {
            base: 78.0,
            amp: 8.0,
            seed: 31,
        }),
    },
    PointSpec {
        equip: "ahu-3",
        slug: "cooling-valve",
        display_name: "Cooling Valve",
        kind: PointKindSpec::Cmd,
        unit: "%",
        tags: &["cool", "valve", "cmd"],
        cur_str: None,
        cur_num: None,
        slots: &[(13, 96.0), (16, 40.0)],
        relinquish_default: None,
        curve: Some(Curve {
            base: 60.0,
            amp: 30.0,
            seed: 41,
        }),
    },
    PointSpec {
        equip: "ahu-3",
        slug: "heating-valve",
        display_name: "Heating Valve",
        kind: PointKindSpec::Cmd,
        unit: "%",
        tags: &["heat", "valve", "cmd"],
        cur_str: None,
        cur_num: None,
        slots: &[(16, 35.0)],
        relinquish_default: None,
        curve: Some(Curve {
            base: 20.0,
            amp: 18.0,
            seed: 51,
        }),
    },
    PointSpec {
        equip: "ahu-3",
        slug: "discharge-sp",
        display_name: "Discharge Temp Setpoint",
        kind: PointKindSpec::Sp,
        unit: "°C",
        tags: &["discharge", "temp", "sp"],
        cur_str: None,
        cur_num: None,
        slots: &[(10, 13.0), (16, 14.0)],
        relinquish_default: Some(14.0),
        curve: Some(Curve {
            base: 13.0,
            amp: 0.3,
            seed: 61,
        }),
    },
    PointSpec {
        equip: "ahu-3",
        slug: "occupancy",
        display_name: "Zone Occupancy",
        kind: PointKindSpec::Sensor,
        unit: "",
        tags: &["zone", "occ", "sensor"],
        cur_str: Some("Occupied"),
        cur_num: None,
        slots: &[],
        relinquish_default: None,
        curve: None,
    },
    sensor(
        "ahu-3",
        "static-press",
        "Duct Static Pressure",
        "Pa",
        &["duct", "pressure", "sensor"],
        248.0,
        Curve {
            base: 250.0,
            amp: 14.0,
            seed: 81,
        },
    ),
    sensor(
        "ahu-1",
        "discharge-temp",
        "Discharge Air Temp",
        "°C",
        &["discharge", "air", "temp", "sensor"],
        13.0,
        Curve {
            base: 13.1,
            amp: 1.0,
            seed: 13,
        },
    ),
    PointSpec {
        equip: "ahu-1",
        slug: "supply-fan-cmd",
        display_name: "Supply Fan Speed",
        kind: PointKindSpec::Cmd,
        unit: "%",
        tags: &["supply", "fan", "cmd"],
        cur_str: None,
        cur_num: None,
        slots: &[(16, 64.0)],
        relinquish_default: None,
        curve: Some(Curve {
            base: 62.0,
            amp: 9.0,
            seed: 33,
        }),
    },
    sensor(
        "chiller-1",
        "chw-supply-temp",
        "CHW Supply Temp",
        "°C",
        &["chw", "cool", "temp", "sensor"],
        6.8,
        Curve {
            base: 6.6,
            amp: 0.5,
            seed: 91,
        },
    ),
    sensor(
        "chiller-1",
        "load-pct",
        "Chiller Load",
        "%",
        &["cool", "load", "sensor"],
        72.0,
        Curve {
            base: 68.0,
            amp: 16.0,
            seed: 93,
        },
    ),
    sensor(
        "meter-main",
        "kw-total",
        "Total Demand",
        "kW",
        &["elec", "meter", "energy", "kw"],
        412.0,
        Curve {
            base: 360.0,
            amp: 120.0,
            seed: 7,
        },
    ),
    sensor(
        "meter-main",
        "kw-chillers",
        "Chillers",
        "kW",
        &["elec", "submeter", "energy"],
        168.0,
        Curve {
            base: 150.0,
            amp: 50.0,
            seed: 101,
        },
    ),
    sensor(
        "meter-main",
        "kw-ahus",
        "AHUs / Fans",
        "kW",
        &["elec", "submeter", "energy"],
        96.0,
        Curve {
            base: 90.0,
            amp: 24.0,
            seed: 103,
        },
    ),
    sensor(
        "meter-main",
        "kw-lighting",
        "Lighting",
        "kW",
        &["elec", "submeter", "energy"],
        64.0,
        Curve {
            base: 60.0,
            amp: 18.0,
            seed: 105,
        },
    ),
    sensor(
        "meter-main",
        "kw-plug",
        "Plug loads",
        "kW",
        &["elec", "submeter", "energy"],
        52.0,
        Curve {
            base: 50.0,
            amp: 10.0,
            seed: 107,
        },
    ),
    sensor(
        "meter-main",
        "kw-other",
        "Other",
        "kW",
        &["elec", "submeter", "energy"],
        32.0,
        Curve {
            base: 30.0,
            amp: 6.0,
            seed: 109,
        },
    ),
    sensor(
        "vav-4-12",
        "comfort-index",
        "Comfort Index",
        "%",
        &["zone", "comfort", "sensor"],
        97.2,
        Curve {
            base: 96.4,
            amp: 1.6,
            seed: 55,
        },
    ),
    PointSpec {
        equip: "vav-4-12",
        slug: "damper-pos",
        display_name: "Damper Position",
        kind: PointKindSpec::Cmd,
        unit: "%",
        tags: &["zone", "damper", "cmd"],
        cur_str: None,
        cur_num: None,
        slots: &[(16, 44.0)],
        relinquish_default: None,
        curve: Some(Curve {
            base: 45.0,
            amp: 12.0,
            seed: 57,
        }),
    },
];

pub const SPARKS: &[SparkSpec] = &[
    SparkSpec {
        rule: "simultaneous-heat-cool",
        severity: SparkSeveritySpec::Fault,
        message: "Simultaneous heating and cooling — cooling valve 96% while heating valve 35%",
        points: &["ahu-3-cooling-valve", "ahu-3-heating-valve"],
        acknowledged: false,
    },
    SparkSpec {
        rule: "rogue-zone",
        severity: SparkSeveritySpec::Fault,
        message: "CHW supply temp 9.2°C above setpoint for 14 min — possible fouling",
        points: &["chiller-1-chw-supply-temp"],
        acknowledged: false,
    },
    SparkSpec {
        rule: "stuck-damper",
        severity: SparkSeveritySpec::Warning,
        message: "Damper command changed 40% but airflow flat — possible stuck actuator",
        points: &["vav-4-12-damper-pos"],
        acknowledged: false,
    },
    SparkSpec {
        rule: "after-hours-runtime",
        severity: SparkSeveritySpec::Warning,
        message: "Fan running 3.2h after scheduled off — 41 kWh waste",
        points: &["ahu-1-supply-fan-cmd"],
        acknowledged: false,
    },
    SparkSpec {
        rule: "sensor-drift",
        severity: SparkSeveritySpec::Info,
        message: "Flue temp sensor drift detected vs sibling sensor (1.8°C)",
        points: &[],
        acknowledged: true,
    },
    SparkSpec {
        rule: "demand-spike",
        severity: SparkSeveritySpec::Warning,
        message: "Peak demand approaching 92% of contracted capacity",
        points: &["meter-main-kw-total"],
        acknowledged: false,
    },
    SparkSpec {
        rule: "low-delta-t",
        severity: SparkSeveritySpec::Info,
        message: "Chilled water ΔT 3.1°C — below 5°C design (low-ΔT syndrome)",
        points: &["chiller-1-load-pct"],
        acknowledged: true,
    },
];
