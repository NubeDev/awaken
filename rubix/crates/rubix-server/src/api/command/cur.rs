//! POST /api/v1/points/{id}/cur — ingest a sensor sample as the current value.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use rubix_core::{HisSample, Point, PointValue};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CurSample {
    pub value: PointValue,
    /// Sample time; defaults to now.
    pub ts: Option<DateTime<Utc>>,
}

#[utoipa::path(post, path = "/api/v1/points/{id}/cur", params(("id" = Uuid, Path)),
    request_body = CurSample, tag = "command",
    responses((status = 200, body = Point), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn ingest_cur(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<CurSample>,
) -> Result<Json<Point>, ApiError> {
    let sample = HisSample {
        ts: req.ts.unwrap_or_else(Utc::now),
        value: req.value,
    };
    let store = state.store.clone();
    let (point, keyexpr) =
        blocking(move || Ok((store.ingest_cur(id, &sample)?, store.point_keyexpr(id)?))).await?;
    if let Some(bus) = &state.bus {
        bus.publish_cur(&keyexpr, point.cur_value.as_ref()).await;
    }
    Ok(Json(point))
}
