//! DELETE /api/v1/widgets/{id} — remove a pinned widget (the builder's Remove).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/widgets/{id}", params(("id" = Uuid, Path)), tag = "widgets",
    responses((status = 204), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_widget(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    blocking(move || Ok(state.store.delete_widget(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
