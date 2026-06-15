//! `/agent/memory` — the agent memory seam over the wire.
//!
//! These routes expose the two `rubix-agent` memory operations, each on the
//! enforcement point AGENT.md mandates:
//!
//! - `POST /agent/memory/recall` runs a nearest-neighbour search on the request's
//!   **scoped session** — a read, no capability, the session's row perms are the
//!   whole fence (contract #2).
//! - `POST /agent/memory/persist` writes a memory **through the gate** as an
//!   `agent-memory-write` command — a mutation, authorized/captured/correlated/
//!   audited like every other (contract #1).
//!
//! Either way the principal is the request's own: an agent (or any principal that
//! holds the grant) calls these as itself, so its memories land in its tenant and
//! are scoped to it. The embedding is supplied by the caller (the brain produces
//! it via [`Brain::embed`](rubix_agent::Brain::embed) when wired); the seam
//! normalizes it on write.

use axum::Json;
use axum::extract::State;
use rubix_agent::{MemoryRecord, persist_memory, recall_memory};

use crate::auth::Authenticated;
use crate::dto::agent::{
    PersistRequest, PersistedDto, RecallRequest, RecalledDto, parse_kind,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// `POST /agent/memory/recall` — recall the `k` memories nearest a probe.
///
/// Runs on the caller's scoped session, so only memories its row perms admit can
/// match. No grant is checked — recall is a read.
pub async fn recall_memory_route(
    auth: Authenticated,
    Json(body): Json<RecallRequest>,
) -> ApiResult<Json<Vec<RecalledDto>>> {
    let hits = recall_memory(&auth.session, &body.probe, body.k)
        .await
        .map_err(map_memory_error)?;
    Ok(Json(hits.into_iter().map(RecalledDto::from).collect()))
}

/// `POST /agent/memory/persist` — persist a memory through the gate.
///
/// The write is authorized against the caller's `agent-memory-write` grant before
/// anything is persisted (fail closed); a caller lacking it gets `403`.
pub async fn persist_memory_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<PersistRequest>,
) -> ApiResult<Json<PersistedDto>> {
    let kind = parse_kind(&body.kind)
        .ok_or_else(|| ApiError::BadRequest(format!("unknown memory kind `{}`", body.kind)))?;
    let memory = MemoryRecord::new(kind, body.text, &body.embedding).map_err(map_memory_error)?;

    let persisted = persist_memory(state.store.raw(), &auth.principal, &memory, None)
        .await
        .map_err(map_memory_error)?;

    Ok(Json(PersistedDto {
        memory_id: persisted.memory_id.to_string(),
        correlation_id: persisted.correlation_id.to_string(),
    }))
}

/// Map an agent memory failure to its transport status.
///
/// A denied write (the principal lacks `agent-memory-write`) is `403`; a rejected
/// embedding (empty/zero magnitude) is `422`; anything else is a server failure.
fn map_memory_error(error: rubix_agent::AgentError) -> ApiError {
    match error {
        rubix_agent::AgentError::MemoryWrite(msg)
            if msg.contains("denied") || msg.contains("not authorized") =>
        {
            ApiError::Forbidden(msg)
        }
        rubix_agent::AgentError::Embedding(msg) => ApiError::Unprocessable(msg),
        other => ApiError::Internal(other.to_string()),
    }
}
