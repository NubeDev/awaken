//! `GET /extensions/<id>/metrics` — merged counters + process gauges.
//!
//! Folds the per-extension counters from the shared
//! [`MetricsRegistry`](rubix_ext::metrics::MetricsRegistry) with the live process
//! gauges off the supervisor handle, via the single projection point
//! `MetricsRegistry::merged`. Every known extension (builtin, wasm, stopped,
//! never-spawned) gets a document — the counters stay meaningful and the process
//! gauges degrade to `null`/zero. An unknown id is a plain `404`.

use axum::Json;
use axum::extract::{Path, State};

use rubix_ext::metrics::ExtensionMetrics;
use rubix_ext::supervisor::LifecycleState;

use crate::auth::Authenticated;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::shared::{ext_id, find_control_record, gauges_for};

/// Read the merged metrics view for an extension.
pub async fn metrics_extension_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
) -> ApiResult<Json<ExtensionMetrics>> {
    // The control record must exist for any answer; an unknown id is a 404.
    find_control_record(&auth.session, &subject)
        .await?
        .ok_or(ApiError::NotFound)?;
    let id = ext_id(&auth, &subject);
    let gauges = gauges_for(&state, &id, LifecycleState::Stopped);
    Ok(Json(state.extensions.metrics.merged(&id, gauges)))
}
