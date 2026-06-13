//! PATCH /api/v1/equips/{id} — edit mutable metadata (`display_name`, `tags`).
//! `path` composes the point keyexpr and is immutable; a body carrying it is
//! rejected with a clear error.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::{Equip, TagSet};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchEquip {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub tags: Option<TagSet>,
    /// Immutable — composes the point keyexpr. Present only to reject changes.
    #[serde(default)]
    pub path: Option<String>,
}

#[utoipa::path(patch, path = "/api/v1/equips/{id}", params(("id" = Uuid, Path)),
    request_body = PatchEquip, tag = "equips",
    responses((status = 200, body = Equip), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_equip(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchEquip>,
) -> Result<Json<Equip>, ApiError> {
    if req.path.is_some() {
        return Err(ApiError::BadRequest(
            "path is immutable (it composes the point keyexpr); \
             rename is delete-and-recreate"
                .into(),
        ));
    }
    if let Some(tags) = &req.tags {
        tags.validate()?;
    }
    let updated = blocking(move || {
        Ok(state
            .store
            .update_equip(id, req.display_name.as_deref(), req.tags.as_ref())?)
    })
    .await?;
    Ok(Json(updated))
}
