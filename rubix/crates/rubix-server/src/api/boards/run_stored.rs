//! POST /api/v1/boards/{slug}/run — evaluate a stored board once on demand.
//!
//! Loads the latest version of `slug` from the store and runs its graph,
//! returning every outport packet — the same result shape as the inline
//! `/boards/run`. Works for any board regardless of trigger, so a `Manual`
//! board (one the scheduler never fires) is run exactly here.

use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use super::run::RunBoardResponse;
use crate::error::{ApiError, ErrorBody};
use crate::flow::StorePointAccess;
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/boards/{slug}/run", tag = "boards",
    params(("slug" = String, Path, description = "Board slug")),
    responses((status = 200, body = RunBoardResponse), (status = 404, body = ErrorBody)))]
pub(crate) async fn run_stored_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<RunBoardResponse>, ApiError> {
    let store = state.store.clone();
    let lookup_slug = slug.clone();
    let board = tokio::task::spawn_blocking(move || store.get_board(&lookup_slug))
        .await
        .map_err(|e| ApiError::Internal(e.into()))??;
    let access = Arc::new(
        StorePointAccess::with_bus(state.store.clone(), state.bus.clone())
            .with_agent(state.agent.clone())
            .with_org(board.graph.tenant_org()),
    );
    let outputs = board
        .graph
        .run(access)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(RunBoardResponse { outputs }))
}
