//! DELETE /api/v1/nav/{id} — remove a node. Children cascade (the `parent_id`
//! self-ref FK is `ON DELETE CASCADE`). Authorizes a `nav_node` write on the node.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_nav_node_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/nav/{id}", tag = "nav",
    params(("id" = String, Path, description = "Nav node id")),
    security(("bearer" = [])),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_nav_node(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<StatusCode, ApiError> {
    let store = state.store.clone();
    let node = blocking(move || Ok(store.get_nav_node(id)?)).await?;
    authorize_nav_node_write(&principal, &state.store, &node.org, Some(id))?;
    blocking(move || Ok(state.store.delete_nav_node(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
