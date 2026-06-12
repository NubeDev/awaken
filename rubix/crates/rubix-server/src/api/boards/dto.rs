//! Wire types shared by the stored-board verbs. `BoardRecord` is the domain
//! type; these shape its create request and its JSON response (the graph and
//! trigger are opaque `Object`s to utoipa — their schemas live in rubix-flow
//! and the scheduler).

use rubix_flow::BoardGraph;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::scheduler::{BoardRecord, Trigger};

/// Create (or republish) a board. A slug that already exists creates a new
/// version; the scheduler runs the latest version of each slug.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBoard {
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

/// A stored board as returned to the caller.
#[derive(Debug, Serialize, ToSchema)]
pub struct BoardView {
    pub id: Uuid,
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
