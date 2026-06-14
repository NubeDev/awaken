//! POST /api/v1/agent/chat — run one turn of the embedded BMS agent.
//!
//! Activates the `rubix` agent on the given thread with the user's message and
//! runs it to completion (tool calls included), returning the assistant's final
//! response. Tool calls read and command real points through the priority array
//! — the agent has no path around the gating the tools enforce.

use awaken_runtime::run::RunActivation;
use awaken_runtime_contract::contract::message::Message;
use axum::extract::State;
use axum::Json;
use rubix_tools::TenantScope;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::agent::{run_and_persist, runtime_for_scope, RunOrigin, RunStatus, AGENT_ID};
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChatRequest {
    /// Conversation thread id; reuse it across turns to continue a session.
    pub thread_id: String,
    /// The operator's message to the agent.
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatResponse {
    /// The agent's final assistant response after any tool calls.
    pub response: String,
    /// How many loop steps the run took.
    pub steps: usize,
    /// How the run ended: `completed` for a normal turn, or
    /// `awaiting_approval` when a tool (e.g. a `write_point` above the agent
    /// priority ceiling) suspended for human approval. The run's id is
    /// returned so an operator surface can resume it.
    pub status: ChatStatus,
    /// Set when `status` is `awaiting_approval`: the suspended run's id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

/// Outcome of a chat turn.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChatStatus {
    /// The turn finished and `response` is the agent's answer.
    Completed,
    /// A tool call suspended for human approval; the run is paused.
    AwaitingApproval,
}

/// The tenant a chat run is confined to: the principal's site when it is pinned
/// to one (`{org}/{site}`), else `None` — a broader principal already passed
/// RBAC for its whole scope, and an edge request has no principal at all.
fn principal_scope(principal: &RequestPrincipal) -> Option<TenantScope> {
    let p = principal.0.as_ref()?;
    let org = p.scope.org.as_deref()?;
    let site = p.scope.site.as_deref()?;
    Some(TenantScope::new(org, site))
}

#[utoipa::path(post, path = "/api/v1/agent/chat", tag = "agent",
    request_body = ChatRequest,
    responses(
        (status = 200, body = ChatResponse),
        (status = 400, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn chat(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    // A site-pinned principal confines the run to its tenant; a broader (org or
    // global) principal — or no principal on edge — runs unscoped, already
    // authorized for its full scope by the middleware.
    let scope = principal_scope(&principal);
    let runtime = runtime_for_scope(&state, scope)
        .map_err(|e| ApiError::BadRequest(format!("build scoped agent: {e}")))?
        .ok_or(ApiError::Unavailable("agent runtime not enabled"))?;

    let activation =
        RunActivation::new(req.thread_id, vec![Message::user(req.message)]).with_agent_id(AGENT_ID);
    let record = run_and_persist(&runtime, &state.store, RunOrigin::Chat, activation)
        .await
        .map_err(|e| ApiError::BadRequest(format!("agent run failed: {e}")))?;

    let (status, run_id) = match record.status {
        RunStatus::Suspended => (ChatStatus::AwaitingApproval, Some(record.id)),
        _ => (ChatStatus::Completed, None),
    };
    Ok(Json(ChatResponse {
        response: record.response,
        steps: record.steps,
        status,
        run_id,
    }))
}
