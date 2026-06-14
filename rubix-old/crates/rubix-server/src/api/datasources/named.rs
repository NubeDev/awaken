//! POST /api/v1/datasources/{id}/named/{name} — invoke an operator-registered
//! named query on a datasource with bound parameters.
//!
//! The caller supplies only the query name and parameters, never SQL text — the
//! SQL is operator-authored in `datasources.json`. This is the same tier the AI
//! uses (docs/design/datasources.md "AI"); exposing it over HTTP lets a
//! dashboard bind a named query without embedding its SQL. Lenient path: a cap
//! breach truncates and is reported, not an error.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;
use utoipa::ToSchema;

use super::registry_or_unavailable;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

use super::run::DatasourceResultBody;

#[derive(Debug, Deserialize, ToSchema)]
pub struct NamedQueryRequest {
    /// Positional bound parameters for the named query's `$1..$N`. The count
    /// must match the query's declared `param_count` or the call is a 400.
    #[serde(default)]
    pub params: Vec<Value>,
}

#[utoipa::path(post, path = "/api/v1/datasources/{id}/named/{name}", tag = "datasources",
    params(
        ("id" = String, Path, description = "Registered datasource id"),
        ("name" = String, Path, description = "Operator-registered named query")),
    request_body = NamedQueryRequest,
    responses(
        (status = 200, body = DatasourceResultBody),
        (status = 400, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)))]
pub(crate) async fn invoke_named(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
    Json(req): Json<NamedQueryRequest>,
) -> Result<Json<DatasourceResultBody>, ApiError> {
    let registry = registry_or_unavailable(&state)?;
    let params = super::parse_params(req.params)?;
    let executor = registry.executor(&id)?;
    let result = executor.invoke_named(&name, &params).await?;
    Ok(Json(super::result_body(result)))
}
