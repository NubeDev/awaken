//! GET /api/v1/dashboards?org=&site_id= — boards under an org, optionally one
//! site. Reads the caller may not see are filtered before the wire.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::Dashboard;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::may_read_dashboard;
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDashboardsQuery {
    /// Org whose boards to list (required).
    pub org: String,
    /// Restrict to one site's boards; omit for the org's overviews + all sites.
    pub site_id: Option<Uuid>,
}

#[utoipa::path(get, path = "/api/v1/dashboards", params(ListDashboardsQuery), tag = "dashboards",
    security(("bearer" = [])),
    responses((status = 200, body = [Dashboard]), (status = 401, body = ErrorBody)))]
pub(crate) async fn list_dashboards(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(q): Query<ListDashboardsQuery>,
) -> Result<Json<Vec<Dashboard>>, ApiError> {
    let store = state.store.clone();
    let dashboards = blocking(move || Ok(store.list_dashboards(&q.org, q.site_id)?)).await?;
    let visible: Vec<Dashboard> = dashboards
        .into_iter()
        .filter(|d| may_read_dashboard(&principal, &state.store, d))
        .collect();
    Ok(Json(visible))
}
