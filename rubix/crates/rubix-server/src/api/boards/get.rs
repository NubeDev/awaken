//! GET /api/v1/boards/{slug} — latest version of one board.

use axum::extract::{Path, State};
use axum::Json;

use super::dto::BoardView;
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/boards/{slug}", tag = "boards",
    params(("slug" = String, Path, description = "Board slug")),
    responses((status = 200, body = BoardView), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<BoardView>, ApiError> {
    let store = state.store.clone();
    let board = blocking(move || Ok(store.get_board(&slug)?)).await?;
    Ok(Json(board.into()))
}
