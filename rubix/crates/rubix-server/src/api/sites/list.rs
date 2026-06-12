//! GET /api/v1/sites — list, optionally filtered to one org.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::Site;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::api::blocking::blocking;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSitesQuery {
    /// Restrict to one org.
    pub org: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/sites", params(ListSitesQuery), tag = "sites",
    responses((status = 200, body = [Site])))]
pub(crate) async fn list_sites(
    State(state): State<AppState>,
    Query(q): Query<ListSitesQuery>,
) -> Result<Json<Vec<Site>>, ApiError> {
    Ok(Json(
        blocking(move || Ok(state.store.list_sites(q.org.as_deref())?)).await?,
    ))
}
