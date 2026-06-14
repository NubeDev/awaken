//! POST /api/v1/his/flush — move aged `his` rows from the SQLite hot tier into
//! the Parquet cold tier. The operator (or a scheduled caller) triggers
//! retention; the union query surface is unaffected — flushed rows still read.

use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ApiError, ErrorBody};
use crate::his::flush_aged;
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct FlushRequest {
    /// Flush rows strictly older than this RFC 3339 instant. Defaults to now,
    /// i.e. flush all current history into the cold tier.
    pub older_than: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FlushResponse {
    /// Rows moved out of SQLite.
    pub rows: usize,
    /// Parquet partition files written.
    pub partitions: usize,
}

#[utoipa::path(post, path = "/api/v1/his/flush", tag = "history",
    request_body = FlushRequest,
    responses(
        (status = 200, body = FlushResponse),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn flush_his(
    State(state): State<AppState>,
    Json(req): Json<FlushRequest>,
) -> Result<Json<FlushResponse>, ApiError> {
    let tier = state
        .his_tier
        .as_ref()
        .ok_or(ApiError::Unavailable("his parquet tier not enabled"))?;
    let cutoff = req.older_than.unwrap_or_else(Utc::now);
    let report = flush_aged(&state.store, tier, cutoff).await?;
    Ok(Json(FlushResponse {
        rows: report.rows,
        partitions: report.partitions,
    }))
}
