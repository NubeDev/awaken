//! `POST /query/batch` — run a whole board's panels in one round trip (§3).
//!
//! The board path issues **one** request keyed by chart id instead of N per-panel
//! requests (`rubix/docs/design/DASHBOARDS-SCOPE.md` §3). The capability check and
//! the DataFusion context build happen once; each statement then runs through the
//! **same** read-only guard and the **same** scoped session as `POST /query` —
//! batching is transport, never a permission shortcut. Errors are **per item**: a
//! single bad panel comes back with its error while the others render, and the
//! HTTP status stays `200` unless the request itself is malformed.

use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use rubix_datasource::span_batch;
use rubix_prefs::UnitSystem;

use crate::auth::Authenticated;
use crate::dto::query::{BatchQueryRequest, BatchQueryResponse, BatchQueryResult};
use crate::error::{ApiError, ApiResult};
use crate::http::query::convert::convert_rows;
use crate::http::query::render::{batches_to_rows, columns_of};
use crate::http::query::run::{caller_units, map_query_error, resolve_query};
use crate::state::AppState;

/// The largest number of statements one batch may carry (§3, "≤ ~50").
const MAX_BATCH: usize = 50;

/// Run every keyed statement against one built context, returning a keyed result
/// per item.
///
/// A request with too many statements (or none) is a `400`; a missing
/// `external-query` grant is `403`. Anything that goes wrong with a single
/// statement — a bad time scope, a guard rejection, a plan/exec failure — is
/// reported in that item's `error` field, not as a request failure.
pub async fn run_batch_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<BatchQueryRequest>,
) -> ApiResult<Json<BatchQueryResponse>> {
    if body.queries.is_empty() {
        return Err(ApiError::BadRequest("a batch must carry at least one query".to_owned()));
    }
    if body.queries.len() > MAX_BATCH {
        return Err(ApiError::BadRequest(format!(
            "a batch may carry at most {MAX_BATCH} queries, got {}",
            body.queries.len()
        )));
    }

    // Resolve each item up front (time-macro expansion + carry its quantity map).
    // A resolution failure is this item's error, not the whole batch's — so we
    // carry an `Outcome` per item: a statement to run, or its error to report.
    let mut keys = Vec::with_capacity(body.queries.len());
    let mut outcomes = Vec::with_capacity(body.queries.len());
    let mut any_quantities = false;
    for item in body.queries {
        let (key, request) = item.into_request();
        keys.push(key);
        outcomes.push(match resolve_query(request) {
            Ok(resolved) => {
                any_quantities |= resolved.quantities.is_some();
                Outcome::Run(resolved.sql, resolved.quantities)
            }
            Err(error) => Outcome::Failed(error.to_string()),
        });
    }

    // Run only the statements that resolved, against one shared context.
    let to_run: Vec<String> = outcomes
        .iter()
        .filter_map(|o| match o {
            Outcome::Run(sql, _) => Some(sql.clone()),
            Outcome::Failed(_) => None,
        })
        .collect();

    let ran = if to_run.is_empty() {
        Vec::new()
    } else {
        span_batch(
            &*state.datasources.read().await,
            state.store.raw(),
            &auth.session,
            &state.context_cache,
            &to_run,
        )
        .await
        .map_err(map_query_error)?
    };
    let mut ran = ran.into_iter();

    // Load the caller's unit system once for the whole batch, only if any item
    // declared a quantity to convert (a board of plain panels reads no prefs).
    let units: Option<UnitSystem> = if any_quantities {
        Some(caller_units(&auth).await?)
    } else {
        None
    };

    // Stitch the run results back together with the pre-run failures, in input
    // order, matched by key.
    let mut results = Vec::with_capacity(keys.len());
    for (key, outcome) in keys.into_iter().zip(outcomes) {
        let result = match outcome {
            Outcome::Failed(error) => BatchQueryResult::failed(key, error),
            Outcome::Run(_, quantities) => match ran.next() {
                Some(Ok(batches)) => {
                    let columns = columns_of(&batches);
                    match batches_to_rows(&batches) {
                        Ok(mut rows) => {
                            if let (Some(quantities), Some(units)) = (&quantities, units) {
                                convert_rows(&mut rows, quantities, units);
                            }
                            BatchQueryResult::ok(key, rows, columns)
                        }
                        Err(error) => BatchQueryResult::failed(key, error),
                    }
                }
                Some(Err(error)) => BatchQueryResult::failed(key, error),
                None => BatchQueryResult::failed(
                    key,
                    "internal: missing batch result for statement".to_owned(),
                ),
            },
        };
        results.push(result);
    }

    Ok(Json(BatchQueryResponse { results }))
}

/// Whether an item resolved to a statement to run, or already failed.
enum Outcome {
    /// The resolved SQL to run, plus its post-read quantity-conversion map.
    Run(String, Option<HashMap<String, String>>),
    /// A pre-run failure (e.g. a malformed time scope) to report as-is.
    Failed(String),
}
