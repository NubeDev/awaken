//! GET /api/v1/sites — list, optionally filtered to one org.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::Site;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSitesQuery {
    /// Restrict to one org.
    pub org: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/sites", params(ListSitesQuery), tag = "sites",
    security(("bearer" = [])),
    responses((status = 200, body = [Site]), (status = 401, body = crate::error::ErrorBody)))]
pub(crate) async fn list_sites(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(q): Query<ListSitesQuery>,
) -> Result<Json<Vec<Site>>, ApiError> {
    let sites = blocking(move || Ok(state.store.list_sites(q.org.as_deref())?)).await?;
    // Narrow the listing to the sites the principal may see, so a scoped caller
    // does not learn about sites outside its org/site.
    let visible = sites
        .into_iter()
        .filter(|s| principal.authorize_site_read(&s.org, &s.slug).is_ok())
        .collect();
    Ok(Json(visible))
}
