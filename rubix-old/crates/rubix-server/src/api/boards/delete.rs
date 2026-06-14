//! DELETE /api/v1/boards/{slug}?org=&site_id= — remove every version of a flow
//! within its scope.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use super::dto::BoardScope;
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_board_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/boards/{slug}", tag = "boards",
    params(("slug" = String, Path, description = "Board slug"), BoardScope),
    security(("bearer" = [])),
    responses((status = 204), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_board(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(slug): Path<String>,
    Query(scope): Query<BoardScope>,
) -> Result<StatusCode, ApiError> {
    authorize_board_write(&principal, &state.store, &scope.org, scope.site_id, &slug)?;
    // Resolve the latest-version record first so we can unregister its loop by
    // id (delete drops every version); NotFound if the scope has no such flow.
    let store = state.store.clone();
    let (org, site_id, slug2) = (scope.org.clone(), scope.site_id, slug.clone());
    let board =
        blocking(move || Ok(store.get_board(&org, site_id, &slug2)?)).await?;
    let store = state.store.clone();
    blocking(move || Ok(store.delete_board(&scope.org, scope.site_id, &slug)?)).await?;
    if let Some(scheduler) = &state.scheduler {
        scheduler.unregister(&board);
    }
    Ok(StatusCode::NO_CONTENT)
}
