//! PATCH /api/v1/sites/{id} — edit mutable metadata (`display_name`, `tags`).
//! Identity fields (`org`, `slug`) compose the point keyexpr and are immutable;
//! a body carrying them is rejected (`deny_unknown_fields`).

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::{Site, TagSet};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchSite {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub tags: Option<TagSet>,
    /// Identity fields compose the point keyexpr and are immutable. Present
    /// here only to reject an attempt to change them with a clear error.
    #[serde(default)]
    pub org: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
}

#[utoipa::path(patch, path = "/api/v1/sites/{id}", params(("id" = Uuid, Path)),
    request_body = PatchSite, tag = "sites", security(("bearer" = [])),
    responses((status = 200, body = Site), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_site(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchSite>,
) -> Result<Json<Site>, ApiError> {
    if req.org.is_some() || req.slug.is_some() {
        return Err(ApiError::BadRequest(
            "org/slug are immutable (they compose the point keyexpr); \
             rename is delete-and-recreate"
                .into(),
        ));
    }
    if let Some(tags) = &req.tags {
        tags.validate()?;
    }
    let store = state.store.clone();
    let site = blocking(move || Ok(store.get_site(id)?)).await?;
    principal.authorize_site_write(&site.org, &site.slug)?;
    let updated = blocking(move || {
        Ok(state
            .store
            .update_site(id, req.display_name.as_deref(), req.tags.as_ref())?)
    })
    .await?;
    Ok(Json(updated))
}
