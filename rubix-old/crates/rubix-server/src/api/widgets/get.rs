//! GET /api/v1/widgets/{id} — fetch one pinned widget.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::Widget;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/widgets/{id}", params(("id" = Uuid, Path)), tag = "widgets",
    responses((status = 200, body = Widget), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_widget(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Widget>, ApiError> {
    Ok(Json(
        blocking(move || Ok(state.store.get_widget(id)?)).await?,
    ))
}
