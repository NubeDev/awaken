//! GET /api/v1/equips/{id}

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::Equip;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/equips/{id}", params(("id" = Uuid, Path)), tag = "equips",
    responses((status = 200, body = Equip), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_equip(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Equip>, ApiError> {
    Ok(Json(
        blocking(move || Ok(state.store.get_equip(id)?)).await?,
    ))
}
