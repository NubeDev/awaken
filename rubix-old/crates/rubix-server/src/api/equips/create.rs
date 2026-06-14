//! POST /api/v1/equips — validate path segments/tags, persist.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::{validate_slug, Equip, TagSet};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEquip {
    pub site_id: Uuid,
    /// Slash-separated keyexpr path under the site (`ahu-3` or `ahu-3/fan`).
    pub path: String,
    pub display_name: String,
    #[serde(default)]
    pub tags: TagSet,
}

#[utoipa::path(post, path = "/api/v1/equips", request_body = CreateEquip, tag = "equips",
    responses((status = 201, body = Equip), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_equip(
    State(state): State<AppState>,
    Json(req): Json<CreateEquip>,
) -> Result<(StatusCode, Json<Equip>), ApiError> {
    for segment in req.path.split('/') {
        validate_slug(segment)?;
    }
    req.tags.validate()?;
    let equip = Equip {
        id: Uuid::new_v4(),
        site_id: req.site_id,
        path: req.path,
        display_name: req.display_name,
        tags: req.tags,
        created_at: Utc::now(),
    };
    let stored = equip.clone();
    blocking(move || Ok(state.store.create_equip(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(equip)))
}
