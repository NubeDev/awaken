//! GET /api/v1/boards?org=&site_id= — latest version of every flow in a scope.
//! `?org=` is required; `?site_id=` restricts to one site's flows, else the
//! org's flows at every scope (org-level + all sites). Reads the caller may not
//! see are filtered before the wire.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::dto::BoardView;
use crate::api::blocking::blocking;
use crate::api::scope_auth::may_read_board;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListBoardsQuery {
    pub org: String,
    pub site_id: Option<Uuid>,
}

#[utoipa::path(get, path = "/api/v1/boards", params(ListBoardsQuery), tag = "boards",
    security(("bearer" = [])),
    responses((status = 200, body = [BoardView]), (status = 401, body = ErrorBody)))]
pub(crate) async fn list_boards(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(q): Query<ListBoardsQuery>,
) -> Result<Json<Vec<BoardView>>, ApiError> {
    let store = state.store.clone();
    let boards = blocking(move || Ok(store.latest_boards(&q.org, q.site_id)?)).await?;
    let visible = boards
        .into_iter()
        .filter(|b| may_read_board(&principal, &state.store, &b.org, b.site_id, &b.slug))
        .map(BoardView::from)
        .collect();
    Ok(Json(visible))
}
