//! `POST /readings` — bulk-append readings into the time-series data plane.
//!
//! Readings append, they do not command (`rubix/docs/design/READINGS-TIMESERIES.md`,
//! "Write path — the data plane, never the command gate"). So this is **not**
//! `POST /records`: there is no `apply()`, no audit row, no undo capture per
//! sample. The capability decision is taken **once per request** — a fail-closed
//! `readings-append` grant — mirroring the ingest contract (one decision at
//! subscribe, not per message); after it passes, the batch is written directly on
//! the root/owner handle, never a scoped session (scoped sessions cannot write
//! `reading` at all). The edge partition is the principal's namespace, taken from
//! the authenticated identity and never from the body, so a publisher cannot
//! write into another edge's partition. The deterministic `(series, at)` id makes
//! a re-append (a backfill, a retried request) an idempotent no-op.

use axum::Json;
use axum::extract::State;
use rubix_core::{Reading, append_readings};
use rubix_gate::{Capability, check_capability};
use surrealdb::types::Datetime;

use crate::auth::Authenticated;
use crate::dto::reading::{AppendReadingsRequest, AppendReadingsResponse, ReadingSampleDto};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Append a batch of `{ at, value }` samples for one series, attributed to the
/// principal's edge partition.
///
/// A caller lacking the `readings-append` grant gets `403` before anything is
/// written (fail closed); a malformed sample timestamp is `400`. On success the
/// readings land on the owner handle and the principal's namespace is evicted from
/// the scanned-context cache so a board re-scans on its next tick.
pub async fn append_readings_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<AppendReadingsRequest>,
) -> ApiResult<Json<AppendReadingsResponse>> {
    // One capability decision for the whole batch, up front, fail closed.
    if !check_capability(state.store.raw(), &auth.principal, Capability::ReadingsAppend)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        return Err(ApiError::Forbidden(
            "append requires the readings-append capability".to_owned(),
        ));
    }

    let namespace = auth.principal.namespace.as_str();
    let readings = body
        .samples
        .into_iter()
        .map(|sample| reading_of(namespace, &body.series, sample))
        .collect::<Result<Vec<Reading>, ApiError>>()?;

    let appended = append_readings(state.store.raw(), &readings)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    state.context_cache.invalidate_namespace(namespace);

    Ok(Json(AppendReadingsResponse { appended }))
}

/// Build one [`Reading`] from a request sample, in the principal's partition.
///
/// `at` is parsed from its RFC 3339 string (a malformed instant is a `400`, not a
/// silent drop); `series` and `namespace` are supplied by the caller's request and
/// identity, never the sample. Extras default to an empty object so the row stays
/// lean.
fn reading_of(namespace: &str, series: &str, sample: ReadingSampleDto) -> Result<Reading, ApiError> {
    let at: Datetime = sample
        .at
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("sample `at` is not RFC 3339: {}", sample.at)))?;
    let content = sample.content.unwrap_or_else(|| serde_json::json!({}));
    Ok(Reading::new(namespace, series, at, sample.value, content))
}
