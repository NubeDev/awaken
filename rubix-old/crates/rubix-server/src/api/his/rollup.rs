//! POST /api/v1/his/rollup — time-bucketed aggregate over `his` for one or
//! more points. Dashboards and rule boards use this for trends.

use axum::extract::State;
use axum::Json;
use rubix_query::RollupSpec;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct RollupResponse {
    /// One row per (point_id, bucket): `{point_id, bucket, value, samples}`.
    pub series: Vec<Value>,
}

#[utoipa::path(post, path = "/api/v1/his/rollup", tag = "history",
    request_body = RollupSpec,
    responses(
        (status = 200, body = RollupResponse),
        (status = 400, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn rollup_his(
    State(state): State<AppState>,
    Json(spec): Json<RollupSpec>,
) -> Result<Json<RollupResponse>, ApiError> {
    let engine = state
        .query
        .as_ref()
        .ok_or(ApiError::Unavailable("query engine not enabled"))?;
    let series = engine
        .his_rollup(&spec)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(RollupResponse { series }))
}
