//! GET /api/v1/boards — latest version of every stored board.

use axum::extract::State;
use axum::Json;

use super::dto::BoardView;
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/boards", tag = "boards",
    responses((status = 200, body = [BoardView]), (status = 500, body = ErrorBody)))]
pub(crate) async fn list_boards(
    State(state): State<AppState>,
) -> Result<Json<Vec<BoardView>>, ApiError> {
    let store = state.store.clone();
    let boards = blocking(move || Ok(store.latest_boards()?)).await?;
    Ok(Json(boards.into_iter().map(BoardView::from).collect()))
}
