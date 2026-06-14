//! POST /api/v1/sparks/{id}/ack — acknowledge a finding.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/sparks/{id}/ack", params(("id" = Uuid, Path)), tag = "sparks",
    responses((status = 204), (status = 404, body = ErrorBody)))]
pub(crate) async fn ack_spark(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    blocking(move || Ok(state.store.ack_spark(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
