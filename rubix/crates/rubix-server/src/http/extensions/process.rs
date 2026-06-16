//! `GET /extensions/<id>/process` — live pid + sampled process stats.
//!
//! Process-flavour only. A builtin extension runs inside the host and a wasm
//! component is in-process — both report **no** process. A process-flavour
//! extension that is not currently `Running` (stopped, failed, never spawned)
//! likewise has no live process. Every such case is a `404` carrying the stable
//! code `ext.process.not_running`; an unknown id is a plain `404`. On success the
//! body is the [`ProcessStats`](rubix_ext::supervisor::ProcessStats) shape — a
//! pure projection of the supervisor handle.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::auth::Authenticated;
use crate::state::AppState;

use super::shared::{ext_id, find_control_record, flavour_of};

/// Stable code returned with the `404` when there is no live process.
const NOT_RUNNING: &str = "ext.process.not_running";

#[derive(Serialize)]
struct NotRunning {
    code: &'static str,
}

fn not_running() -> Response {
    (StatusCode::NOT_FOUND, Json(NotRunning { code: NOT_RUNNING })).into_response()
}

/// Read the live process stats for an extension.
pub async fn process_extension_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
) -> Response {
    // The control record must exist for any answer; an unknown id is a plain 404.
    let record = match find_control_record(&auth.session, &subject).await {
        Ok(Some(r)) => r,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return e.into_response(),
    };

    // Builtin / wasm have no host-visible child — report not-running.
    if !flavour_of(&record).reports_process_stats() {
        return not_running();
    }

    let id = ext_id(&auth, &subject);
    match state
        .extensions
        .supervisors
        .get(&id)
        .and_then(|h| h.process_stats())
    {
        Some(stats) => Json(stats).into_response(),
        None => not_running(),
    }
}
