//! POST /api/v1/datasources/{id}/query — run operator-authored native SQL
//! against a registered external datasource and return `{ columns, rows }`.
//!
//! This is the lenient (dashboard/authoring) path: a result that breaches the
//! datasource's caps is truncated and flagged with `breached: true`, not turned
//! into an error. The strict (spark) path lives in the `datasource` board node.
//!
//! The SQL is operator-authored — the same trust tier as a widget definition
//! (docs/design/datasources.md "Query authoring tiers"). The executor still
//! refuses multi-statement input and binds every parameter positionally; values
//! are never spliced into the SQL text.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use super::registry_or_unavailable;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, ToSchema)]
pub struct DatasourceQueryRequest {
    /// A single native SQL statement for the external engine (e.g. a TimescaleDB
    /// `time_bucket` aggregate). Multi-statement input is refused.
    pub sql: String,
    /// Positional bound parameters for `$1..$N`, typed and bound — never spliced
    /// into the SQL. Each is `{ "type": "text"|"int"|"float"|"bool"|"timestamp",
    /// "value": ... }` or `{ "type": "null" }`. Omit for a parameterless query.
    #[serde(default)]
    pub params: Vec<Value>,
}

/// `{ columns, rows, breached }` — the same shape `rubix-query` returns, so a
/// dashboard renders it identically. `breached` is true when the result was
/// truncated at a cap.
#[derive(Debug, Serialize, ToSchema)]
pub struct DatasourceResultBody {
    pub columns: Value,
    pub rows: Value,
    pub breached: bool,
}

#[utoipa::path(post, path = "/api/v1/datasources/{id}/query", tag = "datasources",
    params(("id" = String, Path, description = "Registered datasource id")),
    request_body = DatasourceQueryRequest,
    responses(
        (status = 200, body = DatasourceResultBody),
        (status = 400, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn run_query(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<DatasourceQueryRequest>,
) -> Result<Json<DatasourceResultBody>, ApiError> {
    let registry = registry_or_unavailable(&state)?;
    let params = super::parse_params(req.params)?;
    let executor = registry.executor(&id)?;
    let result = executor.execute(&req.sql, &params).await?;
    Ok(Json(super::result_body(result)))
}
