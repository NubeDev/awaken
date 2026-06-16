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
use rubix_core::Id;
use rubix_datasource::{DatasourceError, span};
use rubix_gate::read_record_on_session;
use rubix_prefs::UnitSystem;
use rubix_query::{
    QueryError, Transform, apply_time_scope, apply_transforms, expand_variables, now_ms,
};

use axum::response::{IntoResponse, Response};

use crate::auth::Authenticated;
use crate::dto::query::{QueryRequest, QueryResponse, QueryVariableDto, TransformDto};
use crate::error::{ApiError, ApiResult};
use crate::http::prefs::read::load_prefs;
use crate::http::query::convert::convert_rows;
use crate::http::query::render::{batches_to_rows, columns_of};
use crate::http::query::stream::promote_query_route;
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
) -> ApiResult<Response> {
    // A streamed query promotes to a Tier-2 job (202 + WS chunks); the quick path is
    // unchanged inline rows (200). The client handles either shape.
    if body.stream {
        return promote_query_route(state, auth, body).await;
    }
    let body = resolve_request_sql(&auth, body).await?;
    let resolved = resolve_query(body).map_err(map_time_error)?;
    let registry = state.datasources.read().await;
    let mut batches = span(
        &registry,
        state.store.raw(),
        &auth.session,
        &state.context_cache,
        &resolved.sql,
    )
    .await
    .map_err(map_query_error)?;

    // Server-side transform tier (§1): aggregate ops run over the result rows
    // before they hit the wire; cosmetic ops are left for the client.
    if resolved.has_aggregate() {
        batches = apply_transforms(batches, &resolved.transforms)
            .await
            .map_err(map_time_error)?;
    }

    let columns = columns_of(&batches);
    let mut rows = batches_to_rows(&batches).map_err(|e| ApiError::Internal(e.to_string()))?;

    if let Some(quantities) = &resolved.quantities {
        convert_rows(&mut rows, quantities, caller_units(&auth).await?);
    }
    Ok(Json(QueryResponse { rows, columns }).into_response())
}

/// A request resolved to its final SQL plus the post-read layers.
pub(crate) struct ResolvedQuery {
    /// The statement to run (time macros already expanded).
    pub sql: String,
    /// The `column → quantity` map to apply after the rows are read, if any.
    pub quantities: Option<HashMap<String, String>>,
    /// The post-query transform pipeline; the aggregate ops run server-side (§1).
    pub transforms: Vec<Transform>,
}

impl ResolvedQuery {
    /// Whether any server-side (aggregate) transform applies — lets a caller skip
    /// the transform stage entirely for the common no-transform path.
    pub fn has_aggregate(&self) -> bool {
        self.transforms.iter().any(Transform::is_aggregate)
    }
}

/// Resolve a request to its final SQL (expanding any time macros) and carry its
/// quantity map + transforms through for the post-read layers.
///
/// `sql` is already the resolved statement (a saved-query id is resolved earlier,
/// on the caller's scope — see [`resolve_request_sql`]). Shared with the batch
/// route so a single statement is resolved identically on both paths. This does
/// not run the read-only guard itself — `span` does, on the returned string — so a
/// macro can never smuggle a second statement past the guard.
pub(crate) fn resolve_query(body: QueryRequest) -> Result<ResolvedQuery, QueryError> {
    let QueryRequest {
        sql,
        query_id: _,
        time,
        quantities,
        transforms,
        variables,
        stream: _,
    } = body;
    let sql = match time {
        Some(time) => {
            let scope = time.into_scope()?;
            apply_time_scope(&sql, &scope, now_ms())?
        }
        None => sql,
    };
    // Lower dashboard variables after the time macros and before the guard (run by
    // `span` on this final string): every value becomes an escaped literal here, so
    // the guard vets the fully-resolved statement (the injection boundary).
    let sql = match variables {
        Some(list) => {
            let resolved = list
                .into_iter()
                .map(QueryVariableDto::into_variable)
                .collect::<Result<Vec<_>, _>>()?;
            expand_variables(&sql, &resolved)?
        }
        None => sql,
    };
    let transforms = match transforms {
        Some(list) => list
            .into_iter()
            .map(TransformDto::into_transform)
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    Ok(ResolvedQuery {
        sql,
        quantities,
        transforms,
    })
}

/// Resolve a request's `query_id` (if any) to SQL **on the caller's scope**,
/// folding it back into the request so [`resolve_query`] proceeds uniformly.
///
/// A saved query is a `kind:"query"` record; reading it through the principal's
/// scoped session means it is only found if the caller may see it, and the SQL it
/// yields then runs under the caller's scope and caps — never the author's (§4b's
/// privilege-escalation guard). A request with no `query_id` is returned as-is.
///
/// # Errors
/// `404` if the id names no readable saved query; `400` if the record is not a
/// well-formed saved query (no `sql`).
pub(crate) async fn resolve_request_sql(
    auth: &Authenticated,
    mut body: QueryRequest,
) -> Result<QueryRequest, ApiError> {
    let Some(query_id) = body.query_id.clone() else {
        return Ok(body);
    };
    let record = read_record_on_session(&auth.session, &Id::from_raw(query_id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    let sql = record
        .content
        .get("sql")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("saved query has no `sql`".to_owned()))?;
    body.sql = sql.to_owned();
    Ok(body)
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
