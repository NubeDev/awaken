//! DELETE /api/v1/equips/{id} — cascades to points and history.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/equips/{id}", params(("id" = Uuid, Path)), tag = "equips",
    responses((status = 204), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_equip(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    blocking(move || Ok(state.store.delete_equip(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
