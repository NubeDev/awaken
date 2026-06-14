//! GET /api/v1/runs — list persisted agent runs, newest first, optionally
//! filtered by lifecycle status (e.g. `?status=suspended` for the approval
//! queue).

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use crate::agent::{RunRecord, RunStatus};
use crate::api::blocking::blocking;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListRunsQuery {
    /// Restrict to one lifecycle status: `completed`, `suspended`, `resumed`,
    /// or `cancelled`.
    pub status: Option<RunStatus>,
}

#[utoipa::path(get, path = "/api/v1/runs", params(ListRunsQuery), tag = "runs",
    responses((status = 200, body = [RunRecord])))]
pub(crate) async fn list_runs(
    State(state): State<AppState>,
    Query(q): Query<ListRunsQuery>,
) -> Result<Json<Vec<RunRecord>>, ApiError> {
    Ok(Json(
        blocking(move || Ok(state.store.list_runs(q.status)?)).await?,
    ))
}
