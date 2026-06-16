//! Bulk job-control routes (`BULK-AND-JOBS.md`): submit, poll, and cancel.
//!
//! These are the generic long-running-job endpoints — `POST /bulk/jobs` opens a
//! job, `GET /bulk/jobs/{id}` polls its status, `DELETE /bulk/jobs/{id}` cancels
//! it. The WS observation channel (`GET /ws/jobs/{id}`) is mounted with the other
//! WS routes; the bulk record CRUD entry (`POST /records/bulk`) is mounted with the
//! record resource. One file per route; this barrel merges them into a router.

mod cancel;
mod status;
mod submit;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

use cancel::cancel_job_route;
use status::job_status_route;
use submit::submit_job_route;

/// The bulk job routes mounted under `/bulk/jobs`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bulk/jobs", post(submit_job_route))
        .route(
            "/bulk/jobs/:id",
            get(job_status_route).delete(cancel_job_route),
        )
}
