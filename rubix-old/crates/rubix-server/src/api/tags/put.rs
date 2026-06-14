//! PUT /api/v1/tags/{kind}/{id} — full-replace the entity's tag set. Enforces the
//! entity's own `edit` authz; rejects an unknown/foreign id (the entity's `get`
//! is the existence oracle). The set is replaced wholesale (the editor owns it).

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::EntityTags;
use uuid::Uuid;

use super::{authorize_entity_write, parse_kind};
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(put, path = "/api/v1/tags/{kind}/{id}", request_body = EntityTags, tag = "tags",
    params(("kind" = String, Path, description = "Entity kind (e.g. dashboard)"),
           ("id" = String, Path, description = "Entity id")),
    security(("bearer" = [])),
    responses((status = 200, body = EntityTags), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn put_tags(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
    Json(tags): Json<EntityTags>,
) -> Result<Json<EntityTags>, ApiError> {
    let kind = parse_kind(&kind)?;
    tags.validate()?;
    let org = authorize_entity_write(&state, &principal, kind, id).await?;
    let stored = tags.clone();
    blocking(move || Ok(state.store.replace_entity_tags(&org, kind.as_str(), id, &stored)?)).await?;
    Ok(Json(tags))
}
