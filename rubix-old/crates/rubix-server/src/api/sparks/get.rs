//! GET /api/v1/sparks/{id} — fetch one finding (triage by id).

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::Spark;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/sparks/{id}", params(("id" = Uuid, Path)), tag = "sparks",
    responses((status = 200, body = Spark), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_spark(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Spark>, ApiError> {
    Ok(Json(
        blocking(move || Ok(state.store.get_spark(id)?)).await?,
    ))
}
