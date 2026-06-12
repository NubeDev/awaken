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
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::agent::AGENT_ID;
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
}

#[utoipa::path(post, path = "/api/v1/agent/chat", tag = "agent",
    request_body = ChatRequest,
    responses(
        (status = 200, body = ChatResponse),
        (status = 400, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    let runtime = state
        .agent
        .as_ref()
        .ok_or(ApiError::Unavailable("agent runtime not enabled"))?;

    let activation =
        RunActivation::new(req.thread_id, vec![Message::user(req.message)]).with_agent_id(AGENT_ID);
    let result = runtime
        .run_to_completion(activation)
        .await
        .map_err(|e| ApiError::BadRequest(format!("agent run failed: {e}")))?;

    Ok(Json(ChatResponse {
        response: result.response,
        steps: result.steps,
    }))
}
