//! GET /api/v1/points/{id} — point with its keyexpr identity.

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use super::response::PointResponse;
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/points/{id}", params(("id" = Uuid, Path)), tag = "points",
    responses((status = 200, body = PointResponse), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_point(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PointResponse>, ApiError> {
    Ok(Json(
        blocking(move || {
            let point = state.store.get_point(id)?;
            let keyexpr = state.store.point_keyexpr(id)?;
            Ok(PointResponse { keyexpr, point })
        })
        .await?,
    ))
}
