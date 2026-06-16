//! `DELETE /bulk/jobs/{id}` — cancel a job and revoke its ticket
//! (`BULK-AND-JOBS.md`, Tier-2 step + "Limits").
//!
//! Authorized by the ticket (as `Authorization: Bearer <ticket>`): the holder may
//! stop the job it is observing. Cancellation trips the job's `CancellationToken`
//! (the work loop stops cooperatively, ending the job `Failed { cancelled }`) and
//! revokes the ticket immediately, so no further observer can attach. The job row
//! is left for the sweeper to evict after its grace window, so a racing poll still
//! sees the terminal status rather than a bare `404`.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};

use crate::auth::bearer_token;
use crate::error::{ApiError, ApiResult};
use crate::jobs::resolve_observer;
use crate::state::AppState;

/// Cancel a job and revoke its ticket, returning `204 No Content`.
pub async fn cancel_job_route(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let ticket = bearer_token(&headers)
        .ok_or_else(|| ApiError::Unauthenticated("missing job ticket".to_owned()))?;
    let job = resolve_observer(&state, &job_id, &ticket).await?;

    job.cancel();
    rubix_gate::revoke_job_ticket(state.store.raw(), &job_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
