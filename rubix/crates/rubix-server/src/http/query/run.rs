//! `POST /query` — run a read-only query on the principal's scoped session.
//!
//! The unified DataFusion query surface (`rubix/docs/SCOPE.md`, "DataFusion —
//! query and compute"). The query is gated on the WS-04 `external-query`
//! capability and run through the principal's scoped session
//! (`rubix-query::run_authorized`), so the scanned rows are already bounded by
//! SurrealDB row-level permissions (contracts #1, #2). Result Arrow batches are
//! rendered to JSON rows for the wire.

use axum::Json;
use axum::extract::State;
use rubix_query::run_authorized;

use crate::auth::Authenticated;
use crate::dto::query::{QueryRequest, QueryResponse};
use crate::error::{ApiError, ApiResult};
use crate::http::query::render::batches_to_rows;
use crate::state::AppState;

/// Run the request SQL for the principal, returning the matched rows as JSON.
///
/// A missing `external-query` grant is `403`; a non-read or malformed statement
/// is `400`; an engine failure is `500`.
pub async fn run_query_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<QueryRequest>,
) -> ApiResult<Json<QueryResponse>> {
    let batches = run_authorized(state.store.raw(), &auth.session, &body.sql)
        .await
        .map_err(map_query_error)?;
    let rows = batches_to_rows(&batches).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(QueryResponse { rows }))
}

/// Map a query failure to its transport status.
fn map_query_error(error: rubix_query::QueryError) -> ApiError {
    match error {
        rubix_query::QueryError::Denied => {
            ApiError::Forbidden("query requires the external-query capability".to_owned())
        }
        rubix_query::QueryError::Capability(reason) => ApiError::Forbidden(reason),
        other => ApiError::BadRequest(other.to_string()),
    }
}
