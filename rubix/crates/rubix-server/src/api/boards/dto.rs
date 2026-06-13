//! Wire types shared by the stored-board verbs. `BoardRecord` is the domain
//! type; these shape its create request and its JSON response (the graph and
//! trigger are opaque `Object`s to utoipa — their schemas live in rubix-flow
//! and the scheduler).

use rubix_flow::BoardGraph;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::scheduler::{BoardRecord, Trigger};

/// Create (or republish) a flow. A slug that already exists *within the same
/// scope* creates a new version; the scheduler runs the latest version per
/// scope. A flow is owned by an `org` and optionally a `site` (null = org-level,
/// applying across the org) — the uniform scope dashboards/rules share.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBoard {
    pub org: String,
    /// The single site this flow is for; omit for an org-level flow.
    #[serde(default)]
    pub site_id: Option<Uuid>,
    pub slug: String,
    pub display_name: String,
    /// Defaults to true; a disabled board is stored but never fired.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[schema(value_type = Object)]
    pub trigger: Trigger,
    #[schema(value_type = Object)]
    pub board: BoardGraph,
}

fn default_enabled() -> bool {
    true
}

/// Scope query for the slug-addressed board verbs (`get`/`patch`/`delete`/
/// `run`/`outputs`): `?org=` is required, `?site_id=` optional (omit = the
/// org-level flow of that slug).
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct BoardScope {
    pub org: String,
    #[serde(default)]
    pub site_id: Option<Uuid>,
}

/// Patch mutable metadata on the latest version of a board slug. `slug`,
/// `trigger`, and `graph` define the version — changing those is a new
/// `create_board` (a republish), not a PATCH.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchBoard {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// A stored board as returned to the caller.
#[derive(Debug, Serialize, ToSchema)]
pub struct BoardView {
    pub id: Uuid,
    pub org: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_id: Option<Uuid>,
    pub slug: String,
    pub version: i64,
    pub display_name: String,
    pub enabled: bool,
    #[schema(value_type = Object)]
    pub trigger: Trigger,
    #[schema(value_type = Object)]
    pub graph: BoardGraph,
    pub created_at: String,
}

impl From<BoardRecord> for BoardView {
    fn from(r: BoardRecord) -> Self {
        BoardView {
            id: r.id,
            org: r.org,
            site_id: r.site_id,
            slug: r.slug,
            version: r.version,
            display_name: r.display_name,
            enabled: r.enabled,
            trigger: r.trigger,
            graph: r.graph,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}
