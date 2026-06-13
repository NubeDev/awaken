//! POST /api/v1/query — run a read-only SQL statement over the store.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use rubix_query::QueryVariable;

use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// DataFusion SQL over the canonical tables: `sites`, `equips`, `points`,
    /// `his`, `sparks`. May reference dashboard variables as `$name` / `${name}`
    /// / `${name:csv}` / `${name:singlequote}` / `$__sqlIn(name)`; each is
    /// lowered to a bound parameter before execution (never spliced into SQL).
    pub sql: String,
    /// Variable bindings for the `$name` tokens in `sql`. Omit for a query with
    /// no variables — behaviour is then unchanged. Every value binds as a
    /// parameter; a value can never execute as SQL
    /// (docs/design/variables-and-templating.md §2).
    #[serde(default)]
    pub variables: Vec<QueryVariable>,
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
        .query_with_variables(&req.sql, &req.variables)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(QueryResponse { rows }))
}
