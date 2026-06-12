//! POST /api/v1/runs/{id}/cancel — an operator rejects a suspended run. The
//! held write is discarded and the run transitions to `cancelled`; the store
//! (and the point) are untouched. A run that is not suspended is a conflict.

use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::agent::RunStatus;
use crate::api::blocking::blocking;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/runs/{id}/cancel", params(("id" = String, Path)), tag = "runs",
    responses((status = 204), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn cancel_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    blocking(move || {
        state.store.settle_suspended_run(&id, RunStatus::Cancelled)?;
        Ok(())
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
