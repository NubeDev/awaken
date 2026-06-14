//! Agent-run operator surface — wiring only. Lists persisted runs and lets an
//! operator approve (resume) or reject (cancel) a run suspended in the HITL
//! escalation band. A resumed run re-applies the held `write_point` command
//! through the priority array; a cancelled run discards it (store untouched).

pub(crate) mod cancel;
pub(crate) mod get;
pub(crate) mod list;
pub(crate) mod resume;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/runs", get(list::list_runs))
        .route("/api/v1/runs/{id}", get(get::get_run))
        .route("/api/v1/runs/{id}/resume", post(resume::resume_run))
        .route("/api/v1/runs/{id}/cancel", post(cancel::cancel_run))
}
