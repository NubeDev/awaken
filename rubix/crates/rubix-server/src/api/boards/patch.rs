//! PATCH /api/v1/boards/{slug}?org=&site_id= — edit metadata (`display_name`,
//! `enabled`) on the latest version within scope. Republishing the graph/trigger
//! is a new POST version, not a PATCH. Toggling `enabled` takes effect
//! immediately: the board's scheduler loop is (un)registered to match.

use axum::extract::{Path, Query, State};
use axum::Json;

use super::dto::{BoardScope, BoardView, PatchBoard};
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_scope_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(patch, path = "/api/v1/boards/{slug}", tag = "boards",
    params(("slug" = String, Path, description = "Board slug"), BoardScope),
    request_body = PatchBoard, security(("bearer" = [])),
    responses((status = 200, body = BoardView), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_board(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(slug): Path<String>,
    Query(scope): Query<BoardScope>,
    Json(req): Json<PatchBoard>,
) -> Result<Json<BoardView>, ApiError> {
    authorize_scope_write(&principal, &state.store, &scope.org, scope.site_id)?;
    let store = state.store.clone();
    let board = blocking(move || {
        Ok(store.update_board(
            &scope.org,
            scope.site_id,
            &slug,
            req.display_name.as_deref(),
            req.enabled,
        )?)
    })
    .await?;
    // Reconcile the running loop with the patched state.
    if let Some(scheduler) = &state.scheduler {
        scheduler.register(&board);
    }
    Ok(Json(board.into()))
}
