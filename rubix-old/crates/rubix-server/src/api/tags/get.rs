//! GET /api/v1/tags/{kind}/{id} — the entity's full tag set. Enforces the
//! entity's own read authz; an unknown/foreign id reads back as a 404.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::EntityTags;
use uuid::Uuid;

use super::{authorize_entity_read, parse_kind};
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/tags/{kind}/{id}", tag = "tags",
    params(("kind" = String, Path, description = "Entity kind (e.g. dashboard)"),
           ("id" = String, Path, description = "Entity id")),
    security(("bearer" = [])),
    responses((status = 200, body = EntityTags), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn get_tags(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
) -> Result<Json<EntityTags>, ApiError> {
    let kind = parse_kind(&kind)?;
    let org = authorize_entity_read(&state, &principal, kind, id).await?;
    let tags = blocking(move || Ok(state.store.entity_tags(&org, kind.as_str(), id)?)).await?;
    Ok(Json(tags))
}
