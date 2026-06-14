//! GET /api/v1/runs/{id} — one persisted agent run, including the held write
//! for a run still suspended for approval.

use axum::extract::{Path, State};
use axum::Json;

use crate::agent::RunRecord;
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/runs/{id}", params(("id" = String, Path)), tag = "runs",
    responses((status = 200, body = RunRecord), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RunRecord>, ApiError> {
    Ok(Json(blocking(move || Ok(state.store.get_run(&id)?)).await?))
}
