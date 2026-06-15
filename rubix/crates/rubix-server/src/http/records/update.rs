//! `PATCH /records/:id` — replace a record's content through the gate.
//!
//! An update is a mutation, so it crosses the WS-05 gate: capability grant
//! checked, before/after captured atomically, correlation id minted, write
//! applied, audit row appended (contracts #1, #3, #4). The handler extracts the
//! id and new content, drives the command, and returns the stored record.

use axum::Json;
use axum::extract::{Path, State};
use rubix_core::{Id, read_record};
use rubix_gate::{Change, Command, apply};

use crate::auth::Authenticated;
use crate::dto::record::{RecordDto, UpdateRecordRequest};
use crate::error::{ApiError, ApiResult};
use crate::http::records::capability::RECORD_WRITE;
use crate::http::records::create::{invalidate_scanned_context, map_gate_error};
use crate::state::AppState;

/// Replace record `id`'s content with the request content, through the gate.
pub async fn update_record_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
    Json(body): Json<UpdateRecordRequest>,
) -> ApiResult<Json<RecordDto>> {
    let id = Id::from_raw(id);
    let command = Command::new(
        auth.principal.clone(),
        RECORD_WRITE,
        id.clone(),
        Change::Update(body.content),
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
