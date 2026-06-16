//! `GET /extensions/<id>` — one extension's full record + merged metrics.
//!
//! The control record content (the extension's gated config + last lifecycle
//! action) joined with the merged [`ExtensionMetrics`] view — the same bytes
//! `GET /extensions/<id>/metrics` serves. An id with no control record visible to
//! the caller is a plain `404` (it does not exist, or not in this namespace).

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use rubix_ext::metrics::ExtensionMetrics;
use rubix_ext::supervisor::LifecycleState;

use crate::auth::Authenticated;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::shared::{ext_id, find_control_record, gauges_for};

/// The `GET /extensions/<id>` body.
#[derive(Debug, Serialize)]
pub struct ExtensionDetail {
    /// The extension principal subject.
    pub id: String,
    /// The namespace the extension is scoped to.
    pub namespace: String,
    /// The control record content — gated config, runtime spec, lifecycle field.
    pub content: serde_json::Value,
    /// The merged process + counter metrics view.
    pub metrics: ExtensionMetrics,
}

/// Read one extension's detail.
pub async fn get_extension_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
) -> ApiResult<Json<ExtensionDetail>> {
    let record = find_control_record(&auth.session, &subject)
        .await?
        .ok_or(ApiError::NotFound)?;
    let id = ext_id(&auth, &subject);
    let gauges = gauges_for(&state, &id, LifecycleState::Stopped);
    let metrics = state.extensions.metrics.merged(&id, gauges);
    Ok(Json(ExtensionDetail {
        id: subject,
        namespace: record.namespace.clone(),
        content: record.content.clone(),
        metrics,
    }))
}
