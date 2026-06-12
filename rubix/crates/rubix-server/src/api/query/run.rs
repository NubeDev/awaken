//! POST /api/v1/query — run a read-only SQL statement over the store.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// DataFusion SQL over the canonical tables: `sites`, `equips`, `points`,
    /// `his`, `sparks`.
    pub sql: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QueryResponse {
    /// Result rows as JSON objects (column name -> value).
    pub rows: Vec<Value>,
}

#[utoipa::path(post, path = "/api/v1/query", tag = "query",
    request_body = QueryRequest,
    responses(
        (status = 200, body = QueryResponse),
        (status = 400, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn run_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    let engine = state
        .query
        .as_ref()
        .ok_or(ApiError::Unavailable("query engine not enabled"))?;
    let rows = engine
        .query(&req.sql)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(QueryResponse { rows }))
}
