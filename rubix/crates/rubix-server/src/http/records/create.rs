//! `POST /records` — create a record through the WS-05 command gate.
//!
//! A record write is a mutation, so it crosses the gate (`rubix/docs/SCOPE.md`,
//! "Commands go through the gate"): the gate checks the principal's capability
//! grant, captures before/after, mints the correlation id, applies the write, and
//! appends the immutable audit row (contracts #1, #3, #4). The handler stays thin
//! — extract → build the command → apply → map the stored record to its DTO.

use axum::Json;
use axum::extract::State;
use rubix_core::{Id, read_record};
use rubix_gate::{Change, Command, apply};

use crate::auth::Authenticated;
use crate::dto::record::{CreateRecordRequest, RecordDto};
use crate::error::{ApiError, ApiResult};
use crate::http::records::capability::RECORD_WRITE;
use crate::state::AppState;

/// Create a record carrying the request content, attributed to the principal.
///
/// The new record's id is minted server-side; the gate writes it under the
/// principal's namespace (a command has no cross-tenant write path). Returns the
/// stored record as the gate persisted it.
pub async fn create_record_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<CreateRecordRequest>,
) -> ApiResult<Json<RecordDto>> {
    let id = Id::new();
    let command = Command::new(
        auth.principal.clone(),
        RECORD_WRITE,
        id.clone(),
        Change::Create(body.content),
    );
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_gate_error)?;
    invalidate_scanned_context(&state, &auth.principal);

    let stored = read_record(state.store.raw(), &id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(stored.into()))
}

/// Evict the writing principal's namespace from the scanned-context cache (§4a).
///
/// A record write is the data-change signal the cache must honour: every
/// principal scoped to this namespace re-scans on its next query rather than
/// serving rows from before the write until the TTL. Called on the success path
/// of every record mutation (create/update/delete) so the board reflects a write
/// on its next tick.
pub(crate) fn invalidate_scanned_context(state: &AppState, principal: &rubix_core::Principal) {
    state
        .context_cache
        .invalidate_namespace(&principal.namespace);
}

/// Map a gate failure to its transport status: a denied grant is `403`, anything
/// else an internal error (the write path does not 404 — the id is fresh).
pub(crate) fn map_gate_error(error: rubix_gate::GateError) -> ApiError {
    match error {
        rubix_gate::GateError::CommandDenied(reason) => ApiError::Forbidden(reason),
        rubix_gate::GateError::Validation(reason) => ApiError::Unprocessable(reason),
        other => ApiError::Internal(other.to_string()),
    }
}
