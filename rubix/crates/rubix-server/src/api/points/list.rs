//! GET /api/v1/points — list with equip, site, and tag filters.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::Point;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::api::tag_query::parse_tags;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPointsQuery {
    pub equip_id: Option<Uuid>,
    pub site_id: Option<Uuid>,
    /// Comma-separated tags; matches points carrying all of them.
    pub tags: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/points", params(ListPointsQuery), tag = "points",
    responses((status = 200, body = [Point])))]
pub(crate) async fn list_points(
    State(state): State<AppState>,
    Query(q): Query<ListPointsQuery>,
) -> Result<Json<Vec<Point>>, ApiError> {
    let tags = parse_tags(q.tags.as_deref());
    Ok(Json(
        blocking(move || Ok(state.store.list_points(q.equip_id, q.site_id, &tags)?)).await?,
    ))
}
