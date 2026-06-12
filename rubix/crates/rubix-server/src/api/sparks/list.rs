//! GET /api/v1/sparks — list findings with site/rule/ack filters.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::Spark;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSparksQuery {
    pub site_id: Option<Uuid>,
    pub rule: Option<String>,
    pub acknowledged: Option<bool>,
}

#[utoipa::path(get, path = "/api/v1/sparks", params(ListSparksQuery), tag = "sparks",
    responses((status = 200, body = [Spark])))]
pub(crate) async fn list_sparks(
    State(state): State<AppState>,
    Query(q): Query<ListSparksQuery>,
) -> Result<Json<Vec<Spark>>, ApiError> {
    Ok(Json(
        blocking(move || {
            Ok(state
                .store
                .list_sparks(q.site_id, q.rule.as_deref(), q.acknowledged)?)
        })
        .await?,
    ))
}
