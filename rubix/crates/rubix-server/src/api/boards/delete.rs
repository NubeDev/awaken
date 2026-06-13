//! DELETE /api/v1/boards/{slug} — remove every version of a board.

use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/boards/{slug}", tag = "boards",
    params(("slug" = String, Path, description = "Board slug")),
    responses((status = 204), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<StatusCode, ApiError> {
    let store = state.store.clone();
    let lookup_slug = slug.clone();
    blocking(move || Ok(store.delete_board(&lookup_slug)?)).await?;
    // Stop any running loop and drop its cached outputs for the deleted board.
    if let Some(scheduler) = &state.scheduler {
        scheduler.unregister(&slug);
    }
    Ok(StatusCode::NO_CONTENT)
}
