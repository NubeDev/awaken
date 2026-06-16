//! `POST /extensions/<id>/health` — a real liveness probe.
//!
//! For a process-flavour extension the answer is the **supervisor's** liveness
//! (is the child `Running`?), not a `RETURN true` on the session that only proves
//! the session is signed in (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase 5).
//! A builtin extension (no supervisor) falls back to the session ping. The verdict
//! is `healthy`/`unhealthy`; a probe that itself fails is a `500`.

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use rubix_ext::HealthStatus;
use rubix_ext::runtime::probe_extension_health;

use crate::auth::Authenticated;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::shared::{ext_id, find_control_record};

/// The `POST /extensions/<id>/health` body.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// `healthy` when the extension is live, `unhealthy` otherwise.
    pub status: &'static str,
}

/// Probe an extension's liveness.
pub async fn health_extension_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
) -> ApiResult<Json<HealthResponse>> {
    find_control_record(&auth.session, &subject)
        .await?
        .ok_or(ApiError::NotFound)?;
    let id = ext_id(&auth, &subject);
    let status = probe_extension_health(&state.extensions, &id, &auth.session)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let status = match status {
        HealthStatus::Healthy => "healthy",
        HealthStatus::Unhealthy => "unhealthy",
    };
    Ok(Json(HealthResponse { status }))
}
