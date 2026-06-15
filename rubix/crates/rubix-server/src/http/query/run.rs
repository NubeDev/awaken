//! `POST /query` — run a read-only query spanning SurrealDB and the datasources.
//!
//! The unified DataFusion query surface (`rubix/docs/SCOPE.md`, "DataFusion —
//! query and compute"). The query is gated on the WS-04 `external-query`
//! capability and run through the principal's scoped session, so the SurrealDB
//! rows are bounded by row-level permissions (contracts #1, #2). It spans the
//! shared datasource registry via `rubix_datasource::span`, so a query addresses
//! a registered external connector's tables as `"<datasource id>"."<table>"`
//! alongside the native records. Result Arrow batches are rendered to JSON rows.

use axum::Json;
use axum::extract::State;
use rubix_datasource::{DatasourceError, span};

use crate::auth::Authenticated;
use crate::dto::query::{QueryRequest, QueryResponse};
use crate::error::{ApiError, ApiResult};
use crate::http::query::render::batches_to_rows;
use crate::state::AppState;

/// Run the request SQL for the principal across the unified surface, as JSON rows.
///
/// A missing `external-query` grant is `403`; a non-read or malformed statement
/// is `400`; an unreachable external datasource or engine failure is `500`.
pub async fn run_query_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<QueryRequest>,
) -> ApiResult<Json<QueryResponse>> {
    let registry = state.datasources.read().await;
    let batches = span(&registry, state.store.raw(), &auth.session, &body.sql)
        .await
        .map_err(map_query_error)?;
    let rows = batches_to_rows(&batches).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(QueryResponse { rows }))
}

/// Map a span/query failure to its transport status.
fn map_query_error(error: DatasourceError) -> ApiError {
    match error {
        DatasourceError::Denied => {
            ApiError::Forbidden("query requires the external-query capability".to_owned())
        }
        DatasourceError::Capability(reason) => ApiError::Forbidden(reason),
        DatasourceError::Query(reason) => ApiError::BadRequest(reason),
        other => ApiError::BadRequest(other.to_string()),
    }
}
