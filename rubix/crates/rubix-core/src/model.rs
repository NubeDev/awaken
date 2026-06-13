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

/// What a pinned widget renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    /// Live current value of a point (`target` is a point keyexpr).
    PointValue,
    /// Time-series history of a point (`target` is a point keyexpr).
    PointHistory,
    /// Latest output of a stored board (`target` is a board slug).
    BoardOutput,
    /// A grid read from an external SQL datasource (`target` is the datasource
    /// id, `query` carries the operator-authored native SQL). The same
    /// `{ columns, rows }` shape a point/board tile renders, sourced from a
    /// TimescaleDB/Postgres historian (docs/design/datasources.md "Consumers").
    Datasource,
}

/// A named board of widgets. A dashboard is owned by an `org` (the tenant
/// namespace) and is either **site-scoped** (a single site's board) or an
/// **org overview** that spans every site under the org — `site_id` is `None`
/// for an overview. Tiles carry full point keyexprs, so an overview can mix
/// points from many sites without the dashboard itself binding to one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Dashboard {
    pub id: Uuid,
    /// Owning org namespace (the tenant key). Always set.
    pub org: String,
    /// The single site this board is for; `None` makes it an org overview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_id: Option<Uuid>,
    /// URL-safe identity within the org, unique per `(org, site_id)`.
    pub slug: String,
    /// Human-facing board name.
    pub title: String,
    pub created_at: DateTime<Utc>,
}

impl Dashboard {
    /// True when this board spans every site under its org (no single site).
    pub fn is_overview(&self) -> bool {
        self.site_id.is_none()
    }
}

/// A dashboard tile pinned by an agent (or operator) for later viewing. The
/// agent `pin_widget` tool creates these so a finding or trend it surfaced
/// during a turn persists on a dashboard instead of scrolling away. A widget
/// belongs to a [`Dashboard`]; `site_id` records the site the tile's target
/// lives under (the site that owns it for cascade-delete), which for an
/// overview board may differ from one tile to the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Widget {
    pub id: Uuid,
    /// The dashboard this tile sits on.
    pub dashboard_id: Uuid,
    pub site_id: Uuid,
    pub kind: WidgetKind,
    /// Human-facing tile title.
    pub title: String,
    /// What the tile points at, per `kind`: a point keyexpr (`point_value`,
    /// `point_history`), a board slug (`board_output`), or a datasource id
    /// (`datasource`).
    pub target: String,
    /// Native SQL for a `datasource` tile (operator-authored, the same trust
    /// tier as a spark node's SQL). `None` for every other kind, which carry
    /// their whole binding in `target`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    pub created_at: DateTime<Utc>,
}
