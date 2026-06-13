//! PATCH /api/v1/boards/{slug} — edit metadata (`display_name`, `enabled`) on
//! the latest version. Republishing the graph/trigger is a new POST version,
//! not a PATCH. Toggling `enabled` takes effect immediately: the board's
//! scheduler loop is (un)registered to match the new state, no restart needed.

use axum::extract::{Path, State};
use axum::Json;

use super::dto::{BoardView, PatchBoard};
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(patch, path = "/api/v1/boards/{slug}", tag = "boards",
    params(("slug" = String, Path, description = "Board slug")),
    request_body = PatchBoard,
    responses((status = 200, body = BoardView), (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(req): Json<PatchBoard>,
) -> Result<Json<BoardView>, ApiError> {
    let store = state.store.clone();
    let board = blocking(move || {
        Ok(store.update_board(&slug, req.display_name.as_deref(), req.enabled)?)
    })
    .await?;
    // Reconcile the running loop with the patched state: `register` starts a
    // newly-enabled board and `is_scheduled() == false` (disabled) unregisters
    // it. Either way the runtime now matches what was just stored.
    if let Some(scheduler) = &state.scheduler {
        scheduler.register(&board);
    }
    Ok(Json(board.into()))
}
