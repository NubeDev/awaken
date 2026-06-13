//! PATCH /api/v1/points/{id} — edit mutable metadata (`display_name`, `tags`,
//! `unit`, `kind`). `slug` composes the point keyexpr and is immutable; a body
//! carrying it is rejected. Value/priority-array mutation is the command path.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::{PointKind, TagSet};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::response::PointResponse;
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchPoint {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub tags: Option<TagSet>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub kind: Option<PointKind>,
    /// Immutable — composes the point keyexpr. Present only to reject changes.
    #[serde(default)]
    pub slug: Option<String>,
}

#[utoipa::path(patch, path = "/api/v1/points/{id}", params(("id" = Uuid, Path)),
    request_body = PatchPoint, tag = "points",
    responses((status = 200, body = PointResponse), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_point(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<PatchPoint>,
) -> Result<Json<PointResponse>, ApiError> {
    if req.slug.is_some() {
        return Err(ApiError::BadRequest(
            "slug is immutable (it composes the point keyexpr); \
             rename is delete-and-recreate"
                .into(),
        ));
    }
    if let Some(tags) = &req.tags {
        tags.validate()?;
    }
    let resp = blocking(move || {
        let point = state.store.update_point(
            id,
            req.display_name.as_deref(),
            req.tags.as_ref(),
            req.unit.as_deref(),
            req.kind,
        )?;
        let keyexpr = state.store.point_keyexpr(id)?;
        Ok(PointResponse { keyexpr, point })
    })
    .await?;
    Ok(Json(resp))
}
