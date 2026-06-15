//! `POST /query` — run a read-only query spanning SurrealDB and the datasources.
//!
//! The unified DataFusion query surface (`rubix/docs/SCOPE.md`, "DataFusion —
//! query and compute"). The query is gated on the WS-04 `external-query`
//! capability and run through the principal's scoped session, so the SurrealDB
//! rows are bounded by row-level permissions (contracts #1, #2). It spans the
//! shared datasource registry via `rubix_datasource::span`, so a query addresses
//! a registered external connector's tables as `"<datasource id>"."<table>"`
//! alongside the native records. Result Arrow batches are rendered to JSON rows.
//!
//! When the request carries a structured time scope (§5,
//! `rubix/docs/design/DASHBOARDS-SCOPE.md`), the backend resolves it to a UTC
//! window + snapped grain and expands the SQL's time macros against it **before**
//! the read-only guard runs — so a board never splices a locale datetime string.

use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use rubix_datasource::{DatasourceError, span};
use rubix_prefs::UnitSystem;
use rubix_query::{QueryError, apply_time_scope, now_ms};

use crate::auth::Authenticated;
use crate::dto::query::{QueryRequest, QueryResponse};
use crate::error::{ApiError, ApiResult};
use crate::http::prefs::read::load_prefs;
use crate::http::query::convert::convert_rows;
use crate::http::query::render::{batches_to_rows, columns_of};
use crate::state::AppState;

/// Run the request SQL for the principal across the unified surface, as JSON rows.
///
/// A missing `external-query` grant is `403`; a non-read or malformed statement
/// (including a malformed time scope) is `400`; an unreachable external datasource
/// or engine failure is `500`. Declared quantity columns are converted to the
/// principal's unit system after the rows are read (§2, post-cache per-caller).
pub async fn run_query_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<QueryRequest>,
) -> ApiResult<Json<QueryResponse>> {
    let resolved = resolve_query(body).map_err(map_time_error)?;
    let registry = state.datasources.read().await;
    let batches = span(
        &registry,
        state.store.raw(),
        &auth.session,
        &state.context_cache,
        &resolved.sql,
    )
    .await
    .map_err(map_query_error)?;
    let columns = columns_of(&batches);
    let mut rows = batches_to_rows(&batches).map_err(|e| ApiError::Internal(e.to_string()))?;

    if let Some(quantities) = &resolved.quantities {
        convert_rows(&mut rows, quantities, caller_units(&auth).await?);
    }
    Ok(Json(QueryResponse { rows, columns }))
}

/// A request resolved to its final SQL plus the per-caller unit-conversion map.
pub(crate) struct ResolvedQuery {
    /// The statement to run (time macros already expanded).
    pub sql: String,
    /// The `column → quantity` map to apply after the rows are read, if any.
    pub quantities: Option<HashMap<String, String>>,
}

/// Resolve a request to its final SQL (expanding any time macros) and carry its
/// quantity map through for post-read conversion.
///
/// Shared with the batch route so a single statement is resolved identically on
/// both paths. The resolution does not run the read-only guard itself — `span`
/// does, on the returned string — so a macro can never smuggle a second statement
/// past the guard.
pub(crate) fn resolve_query(body: QueryRequest) -> Result<ResolvedQuery, QueryError> {
    let QueryRequest {
        sql,
        time,
        quantities,
    } = body;
    let sql = match time {
        Some(time) => {
            let scope = time.into_scope()?;
            apply_time_scope(&sql, &scope, now_ms())?
        }
        None => sql,
    };
    Ok(ResolvedQuery { sql, quantities })
}

/// The requesting principal's unit system, from its stored preferences.
///
/// Loaded per request (cheap scoped read) only when a statement declares a
/// quantity column to convert; a query with no quantities never reads prefs.
pub(crate) async fn caller_units(auth: &Authenticated) -> Result<UnitSystem, ApiError> {
    Ok(load_prefs(auth).await?.units)
}

/// Map a time-scope resolution failure (an unknown grain/token, an inverted
/// window, or a bucket macro without a grain) to a `400`.
pub(crate) fn map_time_error(error: QueryError) -> ApiError {
    ApiError::BadRequest(error.to_string())
}

/// Map a span/query failure to its transport status.
pub(crate) fn map_query_error(error: DatasourceError) -> ApiError {
    match error {
        DatasourceError::Denied => {
            ApiError::Forbidden("query requires the external-query capability".to_owned())
        }
        DatasourceError::Capability(reason) => ApiError::Forbidden(reason),
        DatasourceError::Query(reason) => ApiError::BadRequest(reason),
        other => ApiError::BadRequest(other.to_string()),
    }
}
