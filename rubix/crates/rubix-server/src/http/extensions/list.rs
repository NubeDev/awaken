//! `GET /extensions` — every extension in the caller's namespace.
//!
//! One row per control record visible to the caller's scoped session (so the
//! result is the caller's namespace only), each overlaid with the live supervisor
//! gauges when a supervisor is registered. A row renders directly: identity, the
//! last gated lifecycle action (desired state), the packaging flavour, and the
//! observed runtime state + pid + restart count.

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use rubix_ext::supervisor::{LifecycleState, ProcessFlavour};

use crate::auth::Authenticated;
use crate::error::ApiResult;
use crate::state::AppState;

use super::shared::{ext_id, flavour_of, read_control_records, subject_of};

/// One row in the `GET /extensions` response.
#[derive(Debug, Serialize)]
pub struct ExtensionRow {
    /// The extension principal subject.
    pub id: String,
    /// The last gated lifecycle action written to the control record
    /// (`start`/`stop`/`disable`), or `null` if none has been set.
    pub lifecycle: Option<String>,
    /// The packaging flavour (`process`/`builtin`/`wasm`).
    pub flavour: ProcessFlavour,
    /// The observed runtime state from the live supervisor, or `stopped` when no
    /// supervisor is registered.
    pub state: LifecycleState,
    /// The current child's pid, if running.
    pub pid: Option<u32>,
    /// Cumulative restarts the supervisor has performed.
    pub restarts_total: u64,
}

/// List every extension the caller may see.
pub async fn list_extensions_route(
    State(state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<Vec<ExtensionRow>>> {
    let records = read_control_records(&auth.session).await?;
    let mut rows = Vec::with_capacity(records.len());
    for record in &records {
        let subject = subject_of(record);
        let id = ext_id(&auth, &subject);
        let (observed, pid, restarts) = match state.extensions.supervisors.get(&id) {
            Some(h) => (h.lifecycle_state(), h.pid(), h.restarts_total()),
            None => (LifecycleState::Stopped, None, 0),
        };
        rows.push(ExtensionRow {
            id: subject,
            lifecycle: record
                .content
                .get("lifecycle")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            flavour: flavour_of(record),
            state: observed,
            pid,
            restarts_total: restarts,
        });
    }
    Ok(Json(rows))
}
