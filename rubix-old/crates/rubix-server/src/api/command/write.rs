//! POST /api/v1/points/{id}/write — set a priority-array slot.

use axum::extract::{Path, State};
use axum::Json;
use rubix_core::PointValue;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::apply::command_and_publish;
use super::source::{check_agent_priority, WriteSource};
use crate::api::points::response::PointResponse;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct WriteRequest {
    pub value: PointValue,
    /// Priority slot 1..=16 (1 wins). Defaults to 16, the lowest.
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub source: WriteSource,
}

fn default_priority() -> u8 {
    16
}

#[utoipa::path(post, path = "/api/v1/points/{id}/write", params(("id" = Uuid, Path)),
    request_body = WriteRequest, tag = "command",
    responses((status = 200, body = PointResponse), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn write_point(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<WriteRequest>,
) -> Result<Json<PointResponse>, ApiError> {
    check_agent_priority(&state, req.source, req.priority)?;
    let response = command_and_publish(&state, id, req.priority, Some(req.value)).await?;
    Ok(Json(response))
}
