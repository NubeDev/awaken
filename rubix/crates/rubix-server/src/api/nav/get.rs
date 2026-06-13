//! GET /api/v1/nav/{id} — one nav node, gated on `view` of the **node** (not its
//! board). Opening a dashboard node via `?nav=:id` checks this. A node the caller
//! holds no `view` on reads back as a 404, hiding its existence.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::NavNode;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::api::scope_auth::may_read_nav_node;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/nav/{id}", tag = "nav",
    params(("id" = String, Path, description = "Nav node id")),
    security(("bearer" = [])),
    responses((status = 200, body = NavNode), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn get_nav_node(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<Json<NavNode>, ApiError> {
    let store = state.store.clone();
    let node = blocking(move || Ok(store.get_nav_node(id)?)).await?;
    if !may_read_nav_node(&principal, &state.store, &node.org, node.id) {
        return Err(ApiError::NotFound("nav node"));
    }
    Ok(Json(node))
}
