//! GET /api/v1/widgets — list pinned widgets, optionally by site.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::Widget;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListWidgetsQuery {
    pub site_id: Option<Uuid>,
}

#[utoipa::path(get, path = "/api/v1/widgets", params(ListWidgetsQuery), tag = "widgets",
    responses((status = 200, body = [Widget])))]
pub(crate) async fn list_widgets(
    State(state): State<AppState>,
    Query(q): Query<ListWidgetsQuery>,
) -> Result<Json<Vec<Widget>>, ApiError> {
    Ok(Json(
        blocking(move || Ok(state.store.list_widgets(q.site_id)?)).await?,
    ))
}
