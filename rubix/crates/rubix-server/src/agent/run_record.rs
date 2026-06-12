//! The persisted run row backing the operator surface for agent runs.
//!
//! A [`RunRecord`] is the durable projection of one embedded-agent run — a chat
//! turn or an inbound spark dispatch. It records the run's lifecycle status and,
//! for a run suspended in the HITL escalation band, the [`PendingWrite`] the
//! `write_point` tool held for approval. The resume endpoint re-applies that
//! write through the priority array; cancel discards it. The escalation contract
//! the band enforces lives in `rubix-tools` (`write_point`); this is the
//! persistence half so a suspended run survives the request that raised it.

use chrono::{DateTime, Utc};
use rubix_core::PointValue;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// What raised a run: an operator chat turn or an inbound spark dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunOrigin {
    /// A `POST /api/v1/agent/chat` turn.
    Chat,
    /// An inbound spark finding dispatched to the agent.
    Dispatch,
}

impl RunOrigin {
    fn as_str(self) -> &'static str {
        match self {
            RunOrigin::Chat => "chat",
            RunOrigin::Dispatch => "dispatch",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "chat" => Some(RunOrigin::Chat),
            "dispatch" => Some(RunOrigin::Dispatch),
            _ => None,
        }
    }
}

impl std::fmt::Display for RunOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a run is in its lifecycle. `Suspended` runs await an operator
/// approve/cancel; the other states are terminal projections of how the run
/// ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The run finished normally.
    Completed,
    /// A `write_point` call suspended in the escalation band; awaiting approval.
    Suspended,
    /// An operator approved a suspended run and the held write was applied.
    Resumed,
    /// An operator cancelled a suspended run; the held write was discarded.
    Cancelled,
}

impl RunStatus {
    fn as_str(self) -> &'static str {
        match self {
            RunStatus::Completed => "completed",
            RunStatus::Suspended => "suspended",
            RunStatus::Resumed => "resumed",
            RunStatus::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "completed" => Some(RunStatus::Completed),
            "suspended" => Some(RunStatus::Suspended),
            "resumed" => Some(RunStatus::Resumed),
            "cancelled" => Some(RunStatus::Cancelled),
            _ => None,
        }
    }
}

impl std::fmt::Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The point command a suspended run held for human approval — the escalation
/// band's `write_point` arguments. On resume the operator surface re-applies it
/// through the priority array with the agent ceiling re-checked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PendingWrite {
    /// Point keyexpr: `{org}/{site}/{equip-path}/{point}`.
    pub point: String,
    /// Priority slot 1..=16 the agent requested (inside the escalation band).
    pub priority: u8,
    /// Value to command.
    #[schema(value_type = serde_json::Value)]
    pub value: PointValue,
    /// The agent ceiling in force when the run suspended. A resume re-checks
    /// the request against it so a config change between suspend and approve
    /// cannot widen what the agent could command.
    pub agent_min_priority: u8,
}

/// A persisted agent run and, when suspended, the write it holds for approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RunRecord {
    /// The awaken run id.
    pub id: String,
    /// The agent thread the run executed on.
    pub thread_id: String,
    /// What raised the run.
    pub origin: RunOrigin,
    /// Lifecycle status.
    pub status: RunStatus,
    /// The agent's final assistant response.
    pub response: String,
    /// How many loop steps the run took.
    pub steps: usize,
    /// The held write, set only while `status` is `suspended`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_write: Option<PendingWrite>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips_through_str() {
        for s in [
            RunStatus::Completed,
            RunStatus::Suspended,
            RunStatus::Resumed,
            RunStatus::Cancelled,
        ] {
            assert_eq!(RunStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(RunStatus::parse("bogus"), None);
    }

    #[test]
    fn origin_round_trips_through_str() {
        for o in [RunOrigin::Chat, RunOrigin::Dispatch] {
            assert_eq!(RunOrigin::parse(o.as_str()), Some(o));
        }
        assert_eq!(RunOrigin::parse("bogus"), None);
    }
}
