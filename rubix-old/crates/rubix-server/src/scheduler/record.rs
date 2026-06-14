//! A stored board: the reflow [`BoardGraph`] plus the scheduling metadata the
//! supervisory backend needs to fire it. Persisted in the `boards` table;
//! `slug` + `version` are unique so a board can be republished without losing
//! prior versions (versioned board storage per STACK-DEISGN.md).

use chrono::{DateTime, Utc};
use rubix_flow::BoardGraph;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::trigger::Trigger;

/// One persisted, versioned board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardRecord {
    pub id: Uuid,
    /// Owning org namespace (the tenant key). Always set.
    pub org: String,
    /// The single site this flow is for; `None` makes it an org-level flow that
    /// applies across the org. Mirrors [`rubix_core::Dashboard`] scoping.
    pub site_id: Option<Uuid>,
    /// Stable name across versions; unique per scope `(org, site_id, slug)`.
    pub slug: String,
    pub version: i64,
    pub display_name: String,
    /// When false the scheduler ignores the board even if its trigger fires.
    pub enabled: bool,
    pub trigger: Trigger,
    pub graph: BoardGraph,
    pub created_at: DateTime<Utc>,
}

impl BoardRecord {
    /// True if the scheduler should drive this board (enabled and not manual).
    pub fn is_scheduled(&self) -> bool {
        self.enabled && !matches!(self.trigger, Trigger::Manual)
    }
}
