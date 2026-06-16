//! `DELETE /records/:id` — delete a record through the gate.
//!
//! A delete is a mutation, so it crosses the WS-05 gate: capability grant
//! checked, before-image captured atomically, correlation id minted, write
//! applied, audit row appended (contracts #1, #3, #4). The handler drives the
//! command and returns `204` on success.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use rubix_core::Id;
use rubix_gate::{Change, Command, apply};

use crate::auth::Authenticated;
use crate::error::ApiResult;
use crate::http::records::capability::RECORD_WRITE;
use crate::http::records::create::{invalidate_scanned_context, map_gate_error};
use crate::state::AppState;

/// Delete record `id` through the gate, returning `204 No Content`.
pub async fn delete_record_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let command = Command::new(
        auth.principal.clone(),
        RECORD_WRITE,
        Id::from_raw(id),
        Change::Delete,
    );
    apply(state.store.raw(), &command, None)
        .await
        .map_err(map_gate_error)?;
    invalidate_scanned_context(&state, &auth.principal);
    Ok(StatusCode::NO_CONTENT)
}
