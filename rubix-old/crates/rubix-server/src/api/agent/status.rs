//! GET /api/v1/agent/status — read-only view of the embedded agent's config.
//!
//! The agent is process-global and configured by env vars read once at boot
//! (`RUBIX_AI`, `RUBIX_AI_PROVIDER`, `RUBIX_AI_MODEL_ID`, the priority gate).
//! There is no per-tenant agent and no live reconfiguration — this endpoint
//! exists so an operator surface can show whether the agent is on and how it is
//! gated, not to change it. When `RUBIX_AI != 1` the agent is absent and
//! `enabled` is false; the model fields are then `None`.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct AgentStatus {
    /// Whether the embedded agent is up (`RUBIX_AI=1`). When false, `/agent/chat`
    /// returns 503 and the model fields below are absent.
    pub enabled: bool,
    /// Configured provider (e.g. `openai`); absent when disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Configured model id (e.g. `gpt-4o-mini`); absent when disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Tool-call loop bound per run; absent when disabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rounds: Option<usize>,
    /// Commit ceiling: agent writes at priority >= this apply immediately.
    pub min_priority: u8,
    /// Escalation floor: writes below this are operator-reserved and denied
    /// outright; between floor and ceiling they suspend for human approval.
    pub escalation_floor: u8,
    /// Spark dispatch is wired (agent + bus present). It is on unless explicitly
    /// disabled at boot with `RUBIX_AI_DISPATCH=0`, which this endpoint cannot
    /// observe — so a true here means "ready", not "guaranteed running".
    pub dispatch_ready: bool,
}

#[utoipa::path(get, path = "/api/v1/agent/status", tag = "agent",
    responses((status = 200, body = AgentStatus)))]
pub async fn status(State(state): State<AppState>) -> Json<AgentStatus> {
    let blueprint = state.agent_blueprint.as_ref();
    Json(AgentStatus {
        enabled: state.agent.is_some(),
        provider: blueprint.map(|b| b.provider().to_string()),
        model: blueprint.map(|b| b.model_id().to_string()),
        max_rounds: blueprint.map(|b| b.max_rounds()),
        min_priority: state.ai_min_priority,
        escalation_floor: state.ai_escalation_floor,
        dispatch_ready: state.agent.is_some() && state.bus.is_some(),
    })
}
