//! WebSocket live-query bridge.
//!
//! The realtime surface (`rubix/docs/SCOPE.md`, "Realtime"): a client subscribes
//! over a WebSocket and receives the WS-07 data-change feed, filtered by its
//! principal's scoped session (contract #1). [`subscribe`] opens the subscription
//! on the scoped session; [`bridge`] forwards each change as a JSON frame.

mod bridge;
mod jobs;
mod subscribe;

use axum::Router;
use axum::routing::get;

use crate::state::AppState;

use jobs::subscribe_job_route;
use subscribe::subscribe_records_route;

/// The WebSocket routes: the live-query bridge at `/ws/records` and the job
/// observation channel at `/ws/jobs/{id}` (`BULK-AND-JOBS.md`).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ws/records", get(subscribe_records_route))
        .route("/ws/jobs/:id", get(subscribe_job_route))
}
