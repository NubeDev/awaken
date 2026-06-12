//! GET /api/v1/sites/{id}

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::Site;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/sites/{id}", params(("id" = Uuid, Path)), tag = "sites",
    responses((status = 200, body = Site), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_site(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Site>, ApiError> {
    Ok(Json(blocking(move || Ok(state.store.get_site(id)?)).await?))
}
