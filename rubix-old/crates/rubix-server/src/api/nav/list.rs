//! GET /api/v1/nav?org=… — the org's nav tree, **filtered to nodes the principal
//! holds `view` on** (docs/design/page-context-and-nav.md §6). The filter is the
//! existing per-node grant check; a node the caller cannot see never reaches the
//! wire. Returned flat in tree order (`parent_id`, `sort_order`); the client
//! assembles the nesting.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::NavNode;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::api::blocking::blocking;
use crate::api::scope_auth::may_read_nav_node;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct NavFilter {
    /// The tenant org whose tree to list.
    pub org: String,
}

#[utoipa::path(get, path = "/api/v1/nav", tag = "nav", params(NavFilter),
    security(("bearer" = [])),
    responses((status = 200, body = [NavNode]), (status = 403, body = ErrorBody)))]
pub(crate) async fn list_nav(
    State(state): State<AppState>,
    Query(filter): Query<NavFilter>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<NavNode>>, ApiError> {
    let read_org = filter.org;
    let nodes = {
        let store = state.store.clone();
        blocking(move || Ok(store.list_nav_nodes(&read_org)?)).await?
    };
    let visible = nodes
        .into_iter()
        .filter(|n| may_read_nav_node(&principal, &state.store, &n.org, n.id))
        .collect();
    Ok(Json(visible))
}
