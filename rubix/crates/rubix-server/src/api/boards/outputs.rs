//! GET /api/v1/boards/{slug}/outputs — the latest per-node output values a
//! board has produced, from the scheduler's in-memory cache. An enabled
//! interval/subscription board runs autonomously and its outputs land here on
//! every run, so a client can poll this to see the live values without driving
//! a run itself. Returns an empty list when the scheduler is off or the board
//! has not run since the server started.

use axum::extract::{Path, State};
use axum::Json;

use crate::error::ApiError;
use crate::scheduler::PortOutput;
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/boards/{slug}/outputs", tag = "boards",
    params(("slug" = String, Path, description = "Board slug")),
    responses((status = 200, body = [PortOutput])))]
pub(crate) async fn board_outputs(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Vec<PortOutput>>, ApiError> {
    let outputs = match &state.scheduler {
        Some(scheduler) => scheduler.outputs().latest(&slug),
        None => Vec::new(),
    };
    Ok(Json(outputs))
}
