//! POST /api/v1/runs/{id}/resume — an operator approves a suspended run.
//!
//! Re-applies the held `write_point` command through the priority array and
//! transitions the run to `resumed`. The escalation gate is re-checked at
//! approval time: the held slot must still sit at or above the operator-reserved
//! floor (`RUBIX_AI_ESCALATION_FLOOR`), so a floor raised between suspend and
//! approve closes a write that is now operator-reserved rather than applying it.
//! The settle is atomic and one-shot (the row leaves `suspended` before the
//! write), so a double-resume cannot command the point twice.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::agent::{PendingWrite, RunStatus};
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::store::Store;
use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ResumeResponse {
    /// The resumed run's id.
    pub run_id: String,
    /// The point that was commanded.
    pub point: String,
    /// The priority slot the held write committed to.
    pub priority: u8,
    /// The point's effective value after the write.
    #[schema(value_type = Option<serde_json::Value>)]
    pub effective: Option<rubix_core::PointValue>,
}

#[utoipa::path(post, path = "/api/v1/runs/{id}/resume", params(("id" = String, Path)), tag = "runs",
    responses((status = 200, body = ResumeResponse), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody),
              (status = 409, body = ErrorBody)))]
pub(crate) async fn resume_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ResumeResponse>, ApiError> {
    let store = state.store.clone();
    let floor = state.ai_escalation_floor;
    let response = blocking(move || apply_resume(&store, &id, floor)).await?;
    // `point` is the point's keyexpr (`{org}/{site}/{equip}/{point}`), the same
    // string `cur` is published under — surface the approved value to the mesh.
    if let Some(bus) = &state.bus {
        bus.publish_cur(&response.point, response.effective.as_ref())
            .await;
    }
    Ok(Json(response))
}

/// Settle the run to `resumed` (atomic, one-shot) and apply its held write. The
/// held write is recovered from the row settled out of `suspended`; a run with
/// no held write is a conflict (nothing to re-apply).
fn apply_resume(store: &Store, id: &str, floor: u8) -> Result<ResumeResponse, ApiError> {
    let run = store.settle_suspended_run(id, RunStatus::Resumed)?;
    let Some(write) = run.pending_write else {
        return Err(ApiError::Conflict(format!(
            "run `{id}` suspended without a re-appliable write"
        )));
    };
    let effective = commit_write(store, &write, floor)?;
    Ok(ResumeResponse {
        run_id: run.id,
        point: write.point,
        priority: write.priority,
        effective,
    })
}

/// Apply the held command through the priority array, re-checking the slot is
/// still operator-reachable. The run already left `suspended`, so a failure here
/// surfaces to the operator (the run records `resumed`); the held write is gone,
/// matching the one-shot approval contract.
fn commit_write(
    store: &Store,
    write: &PendingWrite,
    floor: u8,
) -> Result<Option<rubix_core::PointValue>, ApiError> {
    if write.priority < floor {
        return Err(ApiError::Forbidden(format!(
            "priority {} is now operator-reserved (below escalation floor {floor}); \
             the held write cannot be approved",
            write.priority
        )));
    }
    let id = store.point_by_keyexpr(&write.point)?;
    let point = store.command_point(
        id,
        write.priority,
        Some(write.value.clone()),
        chrono::Utc::now(),
    )?;
    Ok(point.cur_value)
}
