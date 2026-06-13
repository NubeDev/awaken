//! POST /api/v1/boards/{slug}/run?org=&site_id= — evaluate a stored flow once on
//! demand within its scope.
//!
//! Loads the latest version of `slug` in the scope and runs its graph, returning
//! every outport packet — the same result shape as the inline `/boards/run`.
//! Works for any board regardless of trigger, so a `Manual` board (one the
//! scheduler never fires) is run exactly here.

use axum::extract::{Path, Query, State};
use axum::Json;
use std::sync::Arc;

use super::dto::BoardScope;
use super::run::RunBoardResponse;
use crate::api::scope_auth::may_read_board;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::flow::StorePointAccess;
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/boards/{slug}/run", tag = "boards",
    params(("slug" = String, Path, description = "Board slug"), BoardScope),
    security(("bearer" = [])),
    responses((status = 200, body = RunBoardResponse), (status = 401, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn run_stored_board(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(slug): Path<String>,
    Query(scope): Query<BoardScope>,
) -> Result<Json<RunBoardResponse>, ApiError> {
    let store = state.store.clone();
    let (org, site_id, lookup_slug) = (scope.org.clone(), scope.site_id, slug.clone());
    let board = tokio::task::spawn_blocking(move || store.get_board(&org, site_id, &lookup_slug))
        .await
        .map_err(|e| ApiError::Internal(e.into()))??;
    if !may_read_board(&principal, &state.store, &board.org, board.site_id, &board.slug) {
        return Err(ApiError::NotFound("board"));
    }
    let access = Arc::new(
        StorePointAccess::with_bus(state.store.clone(), state.bus.clone())
            .with_agent(state.agent.clone())
            .with_org(board.graph.tenant_org())
            .with_site(board.graph.tenant_site())
            .with_datasources(state.datasources.clone()),
    );
    let outputs = board
        .graph
        .run(access)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    // Surface this run's values on the live-outputs endpoint too.
    if let Some(scheduler) = &state.scheduler {
        scheduler
            .outputs()
            .record(&slug, &outputs, chrono::Utc::now().to_rfc3339());
    }
    Ok(Json(RunBoardResponse { outputs }))
}
