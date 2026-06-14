//! GET /api/v1/equips — list with site and tag filters.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::Equip;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::api::tag_query::parse_tags;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListEquipsQuery {
    pub site_id: Option<Uuid>,
    /// Comma-separated tags; matches equips carrying all of them.
    pub tags: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/equips", params(ListEquipsQuery), tag = "equips",
    responses((status = 200, body = [Equip])))]
pub(crate) async fn list_equips(
    State(state): State<AppState>,
    Query(q): Query<ListEquipsQuery>,
) -> Result<Json<Vec<Equip>>, ApiError> {
    let tags = parse_tags(q.tags.as_deref());
    Ok(Json(
        blocking(move || Ok(state.store.list_equips(q.site_id, &tags)?)).await?,
    ))
}
