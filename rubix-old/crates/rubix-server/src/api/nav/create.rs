//! POST /api/v1/nav — create a nav node under an org. Authorizes a `nav_node`
//! write in the org, validates the node shape, the parent (same-org), and a
//! dashboard target's org-scoped existence, then persists.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use rubix_core::NavNode;
use uuid::Uuid;

use super::dto::CreateNavNode;
use super::{validate_parent, validate_target_org};
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_nav_node_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/nav", request_body = CreateNavNode, tag = "nav",
    security(("bearer" = [])),
    responses((status = 201, body = NavNode), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn create_nav_node(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<CreateNavNode>,
) -> Result<(StatusCode, Json<NavNode>), ApiError> {
    authorize_nav_node_write(&principal, &state.store, &req.org, None)?;
    let node = NavNode {
        id: Uuid::new_v4(),
        org: req.org,
        parent_id: req.parent_id,
        title: req.title,
        sort_order: req.sort_order,
        target: req.target,
        context: req.context,
        icon: req.icon,
        accent: req.accent,
    };
    node.validate()?;
    validate_target_org(&state, &node.org, &node.target).await?;
    validate_parent(&state, &node.org, Some(node.id), node.parent_id).await?;
    let stored = node.clone();
    blocking(move || Ok(state.store.create_nav_node(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(node)))
}
