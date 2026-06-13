//! GET /api/v1/boards/{slug}/outputs?org=&site_id= — the latest per-node output
//! values a flow has produced, from the scheduler's in-memory cache. An enabled
//! interval/subscription board runs autonomously and its outputs land here on
//! every run, so a client can poll this to see the live values without driving a
//! run itself. Returns an empty list when the scheduler is off or the board has
//! not run since the server started.
//!
//! The scope query authorizes the read (and resolves the board) before reading
//! the cache, which is keyed by slug.

use axum::extract::{Path, Query, State};
use axum::Json;

use super::dto::BoardScope;
use crate::api::scope_auth::may_read_scope;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::scheduler::PortOutput;
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/boards/{slug}/outputs", tag = "boards",
    params(("slug" = String, Path, description = "Board slug"), BoardScope),
    security(("bearer" = [])),
    responses((status = 200, body = [PortOutput]), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody)))]
pub(crate) async fn board_outputs(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(slug): Path<String>,
    Query(scope): Query<BoardScope>,
) -> Result<Json<Vec<PortOutput>>, ApiError> {
    // Authorize against the queried scope. This reads a cache, not the board
    // row, so a deleted board still returns (an empty, cleared list) rather than
    // 404 — the caller asked "what has this slug produced", and the answer after
    // a delete is "nothing".
    may_read_scope_or_forbid(&principal, &state.store, &scope.org, scope.site_id)?;
    let outputs = match &state.scheduler {
        Some(scheduler) => scheduler.outputs().latest(&slug),
        None => Vec::new(),
    };
    Ok(Json(outputs))
}

/// Authorize a scope read or return 403 (the list endpoints filter silently;
/// this single-slug read fails closed instead).
fn may_read_scope_or_forbid(
    principal: &RequestPrincipal,
    store: &crate::store::Store,
    org: &str,
    site_id: Option<uuid::Uuid>,
) -> Result<(), ApiError> {
    if may_read_scope(principal, store, org, site_id) {
        Ok(())
    } else {
        Err(ApiError::Forbidden(format!(
            "subject may not read flows in org `{org}`"
        )))
    }
}
