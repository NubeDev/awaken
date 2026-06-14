//! GET /api/v1/boards/{slug}?org=&site_id= — latest version of one flow within
//! its scope.

use axum::extract::{Path, Query, State};
use axum::Json;

use super::dto::{BoardScope, BoardView};
use crate::api::blocking::blocking;
use crate::api::scope_auth::may_read_board;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/boards/{slug}", tag = "boards",
    params(("slug" = String, Path, description = "Board slug"), BoardScope),
    security(("bearer" = [])),
    responses((status = 200, body = BoardView), (status = 401, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn get_board(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(slug): Path<String>,
    Query(scope): Query<BoardScope>,
) -> Result<Json<BoardView>, ApiError> {
    let store = state.store.clone();
    let board =
        blocking(move || Ok(store.get_board(&scope.org, scope.site_id, &slug)?)).await?;
    if !may_read_board(&principal, &state.store, &board.org, board.site_id, &board.slug) {
        return Err(ApiError::NotFound("board"));
    }
    Ok(Json(board.into()))
}
