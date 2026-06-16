//! `POST /bulk/jobs` — submit a (synthetic) long-running job (`BULK-AND-JOBS.md`).
//!
//! The spine's generic job-submit door and its proof harness: authenticate →
//! `check_capability(bulk-submit)` once → register a job (over-cap → `429`) → mint
//! a short-TTL ticket → spawn the work → return `202 { job_id, ticket, expires }`.
//! The body requests a trivial counting job (emit `steps` progress frames then
//! complete), which exercises registration → ticket → WS stream → poll → eviction
//! end to end without a real workload — the design's "prove the spine with a
//! trivial synthetic job first".

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::auth::Authenticated;
use crate::dto::job::{JobAcceptedDto, SubmitJobRequest};
use crate::error::ApiResult;
use crate::jobs::{
    ResultTransport, SYNTHETIC_STEP_DELAY, drive, mint_ticket, register_job, require_bulk_submit,
};
use crate::state::AppState;

/// The synthetic job's default and maximum step counts — bounded so a submission
/// cannot ask for an unbounded amount of work.
const DEFAULT_STEPS: u32 = 5;
const MAX_STEPS: u32 = 1000;

/// Submit a synthetic background job and return its observation handle.
///
/// A principal lacking `bulk-submit` gets `403` before anything is registered; a
/// principal already at its running-job cap gets `429`. On success the job runs in
/// the background and the response carries the id + ticket to observe it.
pub async fn submit_job_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<SubmitJobRequest>,
) -> ApiResult<(StatusCode, Json<JobAcceptedDto>)> {
    require_bulk_submit(&state, &auth).await?;

    let steps = u64::from(body.steps.unwrap_or(DEFAULT_STEPS).min(MAX_STEPS));
    // Synthetic statuses are small + bounded, so they buffer for the poll.
    let handle = register_job(&state.jobs, &auth, ResultTransport::Poll, steps).await?;
    let job_id = handle.id().to_owned();

    let ticket = match mint_ticket(&state, &auth, &job_id).await {
        Ok(ticket) => ticket,
        Err(error) => {
            // Leave no un-evictable job behind if the ticket could not be issued.
            handle.fail("ticket issuance failed".to_owned());
            return Err(error);
        }
    };

    drive(handle, move |handle| async move {
        for step in 0..steps {
            // Cooperative cancellation: stop promptly on DELETE / ticket expiry.
            if handle.is_cancelled() {
                return Err("cancelled".to_owned());
            }
            handle.item(format!("step-{step}"), "ok", None, None);
            tokio::time::sleep(SYNTHETIC_STEP_DELAY).await;
        }
        Ok(())
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(JobAcceptedDto {
            job_id,
            ticket: ticket.value,
            expires: ticket.expires,
        }),
    ))
}
