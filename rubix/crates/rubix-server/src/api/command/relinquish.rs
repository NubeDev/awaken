//! DELETE /api/v1/points/{id}/write/{priority} — clear a priority slot.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::api::points::response::PointResponse;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/points/{id}/write/{priority}",
    params(("id" = Uuid, Path), ("priority" = u8, Path, description = "Slot 1..=16 to relinquish")),
    tag = "command",
    responses((status = 200, body = PointResponse), (status = 400, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn relinquish_point(
    State(state): State<AppState>,
    Path((id, priority)): Path<(Uuid, u8)>,
) -> Result<Json<PointResponse>, ApiError> {
    Ok(Json(
        blocking(move || {
            let point = state.store.command_point(id, priority, None, Utc::now())?;
            let keyexpr = state.store.point_keyexpr(id)?;
            Ok(PointResponse { keyexpr, point })
        })
        .await?,
    ))
}
