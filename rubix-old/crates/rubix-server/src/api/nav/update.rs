//! PATCH /api/v1/nav/{id} — update / reorder / reparent a node. Authorizes a
//! `nav_node` write on the node, applies the present fields over the stored row,
//! re-validates shape + dashboard-target org + parent, then persists.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::NavNode;
use uuid::Uuid;

use super::dto::PatchNavNode;
use super::{validate_parent, validate_target_org};
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_nav_node_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(patch, path = "/api/v1/nav/{id}", request_body = PatchNavNode, tag = "nav",
    params(("id" = String, Path, description = "Nav node id")),
    security(("bearer" = [])),
    responses((status = 200, body = NavNode), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn update_nav_node(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    principal: RequestPrincipal,
    Json(patch): Json<PatchNavNode>,
) -> Result<Json<NavNode>, ApiError> {
    let store = state.store.clone();
    let mut node = blocking(move || Ok(store.get_nav_node(id)?)).await?;
    authorize_nav_node_write(&principal, &state.store, &node.org, Some(id))?;

    if let Some(parent_id) = patch.parent_id {
        node.parent_id = parent_id;
    }
    if let Some(title) = patch.title {
        node.title = title;
    }
    if let Some(sort_order) = patch.sort_order {
        node.sort_order = sort_order;
    }
    if let Some(target) = patch.target {
        node.target = target;
    }
    if let Some(context) = patch.context {
        node.context = context;
    }
    if let Some(icon) = patch.icon {
        node.icon = icon;
    }
    if let Some(accent) = patch.accent {
        node.accent = accent;
    }

    node.validate()?;
    validate_target_org(&state, &node.org, &node.target).await?;
    validate_parent(&state, &node.org, Some(node.id), node.parent_id).await?;
    let updated = blocking(move || Ok(state.store.update_nav_node(&node)?)).await?;
    Ok(Json(updated))
}
