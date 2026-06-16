//! `GET /bulk/jobs/{id}` — the fallback status poll (`BULK-AND-JOBS.md`, Tier-2 §3).
//!
//! Authorized by the **ticket** (presented as `Authorization: Bearer <ticket>`),
//! not full principal credentials — a job observer holds only a ticket. The poll
//! returns **status always**, and the buffered terminal **result only for
//! poll-transport jobs** (CRUD per-item statuses). For streamed query jobs it is
//! status-only with `result_transport: "stream"`, so the client knows to consume
//! the WS rather than wait for rows here.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;

use crate::auth::bearer_token;
use crate::dto::job::JobStatusDto;
use crate::error::{ApiError, ApiResult};
use crate::jobs::resolve_observer;
use crate::state::AppState;

/// Poll a job's status (and buffered result, when applicable) using its ticket.
///
/// A missing/invalid/expired ticket is `401`; a job the registry no longer holds
/// (evicted past its grace window, or lost to a restart) is `404`.
pub async fn job_status_route(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Json<JobStatusDto>> {
    let ticket = bearer_token(&headers)
        .ok_or_else(|| ApiError::Unauthenticated("missing job ticket".to_owned()))?;
    let job = resolve_observer(&state, &job_id, &ticket).await?;

    Ok(Json(JobStatusDto::project(
        job_id,
        &job.status(),
        job.transport,
        job.result(),
    )))
}
