use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{PointValue, PriorityArray, TagSet};

/// A building / facility. `org` and `slug` are keyexpr path segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Site {
    pub id: Uuid,
    pub org: String,
    pub slug: String,
    pub display_name: String,
    pub tags: TagSet,
    pub created_at: DateTime<Utc>,
}

/// Equipment under a site (AHU, VAV, meter, plant). `path` is the
/// equip-path keyexpr segment, slash-separated for nesting (`ahu-3/fan`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Equip {
    pub id: Uuid,
    pub site_id: Uuid,
    pub path: String,
    pub display_name: String,
    pub tags: TagSet,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PointKind {
    /// Read-only field input (temp sensor, status).
    Sensor,
    /// Writable output commanded through the priority array.
    Cmd,
    /// Writable setpoint commanded through the priority array.
    Sp,
}

impl PointKind {
    pub fn is_writable(self) -> bool {
        matches!(self, PointKind::Cmd | PointKind::Sp)
    }
}

/// A point: the unit of live value, command, and history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Point {
    pub id: Uuid,
    pub equip_id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub kind: PointKind,
    pub unit: Option<String>,
    pub tags: TagSet,
    /// Command slots; only meaningful for writable kinds.
    pub priority_array: PriorityArray,
    /// Current value: effective command for writable points, last sample for sensors.
    pub cur_value: Option<PointValue>,
    pub cur_ts: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Point {
    /// Zenoh keyexpr identity prefix: `{org}/{site}/{equip-path}/{point}`.
    pub fn keyexpr(org: &str, site_slug: &str, equip_path: &str, point_slug: &str) -> String {
        format!("{org}/{site_slug}/{equip_path}/{point_slug}")
    }
}

/// One history sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct HisSample {
    pub ts: DateTime<Utc>,
    pub value: PointValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SparkSeverity {
    Info,
    Warning,
    Fault,
}

/// A rule finding ("spark"): emitted by rule boards, consumed by the UI and
/// dispatched to awaken agents as jobs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Spark {
    pub id: Uuid,
    pub site_id: Uuid,
    /// Rule identity, the `{rule}` segment of `{org}/{site}/spark/{rule}/**`.
    pub rule: String,
    pub severity: SparkSeverity,
    pub message: String,
    /// Points implicated in the finding.
    pub point_ids: Vec<Uuid>,
    pub ts: DateTime<Utc>,
    pub acknowledged: bool,
}
