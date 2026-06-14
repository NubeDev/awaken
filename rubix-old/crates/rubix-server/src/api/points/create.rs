//! POST /api/v1/points — validate, seed the priority array, persist.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::{validate_slug, Point, PointKind, PointValue, PriorityArray, TagSet};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::response::PointResponse;
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePoint {
    pub equip_id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub kind: PointKind,
    pub unit: Option<String>,
    #[serde(default)]
    pub tags: TagSet,
    /// Value used when every priority slot is empty (writable points only).
    pub relinquish_default: Option<PointValue>,
}

#[utoipa::path(post, path = "/api/v1/points", request_body = CreatePoint, tag = "points",
    responses((status = 201, body = PointResponse), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_point(
    State(state): State<AppState>,
    Json(req): Json<CreatePoint>,
) -> Result<(StatusCode, Json<PointResponse>), ApiError> {
    validate_slug(&req.slug)?;
    req.tags.validate()?;
    if req.relinquish_default.is_some() && !req.kind.is_writable() {
        return Err(ApiError::BadRequest(
            "relinquish_default only applies to writable points".into(),
        ));
    }
    let mut priority_array = PriorityArray::new();
    priority_array.relinquish_default = req.relinquish_default;
    let cur_value = priority_array.effective().map(|(_, v)| v.clone());
    let point = Point {
        id: Uuid::new_v4(),
        equip_id: req.equip_id,
        slug: req.slug,
        display_name: req.display_name,
        kind: req.kind,
        unit: req.unit,
        tags: req.tags,
        priority_array,
        cur_ts: cur_value.as_ref().map(|_| Utc::now()),
        cur_value,
        created_at: Utc::now(),
    };
    let stored = point.clone();
    let keyexpr = blocking(move || {
        state.store.create_point(&stored)?;
        Ok(state.store.point_keyexpr(stored.id)?)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(PointResponse { keyexpr, point })))
}
