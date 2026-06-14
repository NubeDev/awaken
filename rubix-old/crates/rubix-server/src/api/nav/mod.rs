//! Navigation-tree routes (docs/design/page-context-and-nav.md §§4,6). A nav node
//! is org-scoped and nestable; each node mounts a board (with context), a static
//! route, or is a group header. Access is per node via the existing grant model
//! (`nav_node` kind): the tree `GET` is filtered to nodes the caller holds `view`
//! on, and opening a node checks `view` on the node. Wiring only; one file per verb.

pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod dto;
pub(crate) mod get;
pub(crate) mod list;
pub(crate) mod seed;
pub(crate) mod update;

pub(crate) use seed::seed_default_tree;

use axum::routing::{get, post};
use axum::Router;
use rubix_core::NavTarget;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::ApiError;
use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/nav", post(create::create_nav_node).get(list::list_nav))
        .route(
            "/api/v1/nav/{id}",
            get(get::get_nav_node)
                .patch(update::update_nav_node)
                .delete(delete::delete_nav_node),
        )
}

/// Validate a node's target against the caller's org. A `dashboard` target's id
/// must name a board that exists **within `org`** — a bare FK cannot encode
/// org-safety (`dashboards.id` is a global PK with `org` a separate column), so a
/// node cannot mount another tenant's board (docs/design/page-context-and-nav.md
/// §4, "What to prove" #4). Group/route targets need no entity lookup.
pub(crate) async fn validate_target_org(
    state: &AppState,
    org: &str,
    target: &NavTarget,
) -> Result<(), ApiError> {
    if let NavTarget::Dashboard { dashboard_id } = target {
        let id = *dashboard_id;
        let store = state.store.clone();
        let dashboard = blocking(move || Ok(store.get_dashboard(id)?)).await?;
        if dashboard.org != org {
            // Reveal neither existence nor org of a cross-tenant board.
            return Err(ApiError::NotFound("dashboard"));
        }
    }
    Ok(())
}

/// Confirm a parent (when present) exists within the same org, surfacing a clear
/// 404 before the store write. `id` is the node being created/updated (a node may
/// not be its own parent).
pub(crate) async fn validate_parent(
    state: &AppState,
    org: &str,
    id: Option<Uuid>,
    parent_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let Some(parent) = parent_id else {
        return Ok(());
    };
    if Some(parent) == id {
        return Err(ApiError::BadRequest("a nav node cannot parent itself".into()));
    }
    let org = org.to_string();
    let store = state.store.clone();
    let parent_node = blocking(move || Ok(store.get_nav_node(parent)?)).await?;
    if parent_node.org != org {
        return Err(ApiError::NotFound("nav node"));
    }
    Ok(())
}
