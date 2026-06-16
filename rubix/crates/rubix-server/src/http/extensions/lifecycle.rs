//! `POST /extensions/<id>/lifecycle` — start / stop / disable, through the gate.
//!
//! The one mutation on this surface, and it crosses the WS-05 gate exactly like a
//! record write (`rubix/docs/design/ADMIN-API.md`): the capability check
//! ([`ExtensionManage`](rubix_gate::Capability::ExtensionManage)) is the gate's,
//! fail closed, so an out-of-grant call is a `403` before any process is touched,
//! and the transition is audited under the *caller's* authority. After the gated
//! write lands, the bridge drives the supervisor to match and reports the
//! observed state ([`drive_lifecycle`]).
//!
//! Identity handoff (Open question 2): starting a *process*-flavour extension
//! needs the child's run credential, which the gate cannot mint and the server
//! does not store. The operator supplies it once in the request body (`secret`);
//! it is threaded into the child's environment, never persisted. `stop`/`disable`
//! and builtin `start` need no secret.

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use rubix_core::Id;
use rubix_ext::runtime::drive_lifecycle;
use rubix_ext::supervisor::{Identity, LifecycleState, ProcessSpec};
use rubix_ext::{ControlMethod, ControlRequest, ExtError, LifecycleAction};

use crate::auth::Authenticated;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::shared::{ext_id, find_control_record, flavour_of, parse_spec};

/// The `POST /extensions/<id>/lifecycle` request body.
#[derive(Debug, Deserialize)]
pub struct LifecycleBody {
    /// The transition to drive: `start`, `stop`, or `disable`.
    pub action: String,
    /// The child's run credential, required to `start` a process-flavour
    /// extension (threaded into the child env, never stored).
    #[serde(default)]
    pub secret: Option<String>,
}

/// The `POST /extensions/<id>/lifecycle` response.
#[derive(Debug, Serialize)]
pub struct LifecycleResponse {
    /// The correlation id the gate stamped onto the command and audit row.
    pub correlation_id: String,
    /// The transition that was applied.
    pub action: String,
    /// The observed supervisor state after driving it (`null` after stop/disable
    /// or a builtin start, where there is no live handle to report).
    pub state: Option<LifecycleState>,
}

/// Drive an extension's lifecycle transition.
pub async fn lifecycle_extension_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
    Json(body): Json<LifecycleBody>,
) -> ApiResult<Json<LifecycleResponse>> {
    let record = find_control_record(&auth.session, &subject)
        .await?
        .ok_or(ApiError::NotFound)?;
    let action = LifecycleAction::parse(&body.action)
        .ok_or_else(|| ApiError::BadRequest(format!("unknown lifecycle action: {}", body.action)))?;
    let flavour = flavour_of(&record);
    let spec = parse_spec(&record);

    // Starting a process-flavour child needs a runtime spec and a run credential.
    if matches!(action, LifecycleAction::Start) && flavour.reports_process_stats() {
        if spec.is_none() {
            return Err(ApiError::Unprocessable(
                "control record has no `runtime` spec to start".to_owned(),
            ));
        }
        if body.secret.as_deref().unwrap_or_default().is_empty() {
            return Err(ApiError::BadRequest(
                "a `secret` is required to start a process-flavour extension".to_owned(),
            ));
        }
    }

    let id = ext_id(&auth, &subject);
    let identity = Identity {
        namespace: auth.principal.namespace.clone(),
        subject: subject.clone(),
        secret: body.secret.unwrap_or_default(),
    };
    // The control record id is the gate's write target; the spec is only consulted
    // on a process `start` (a placeholder is harmless for the other transitions).
    let target = Id::from_raw(record.id.as_str().to_owned());
    let request = ControlRequest::new(
        ControlMethod::Lifecycle,
        target,
        serde_json::json!({ "action": body.action }),
    );
    let spec = spec.unwrap_or_else(|| ProcessSpec::new(std::path::PathBuf::new()));

    let outcome = drive_lifecycle(
        &state.extensions,
        state.store.raw(),
        &auth.principal,
        &id,
        &request,
        spec,
        identity,
    )
    .await
    .map_err(map_ext_error)?;

    Ok(Json(LifecycleResponse {
        correlation_id: outcome.correlation_id.as_str().to_owned(),
        action: outcome.action.as_str().to_owned(),
        state: outcome.state,
    }))
}

/// Map an extension-runtime error onto the transport's vocabulary.
fn map_ext_error(err: ExtError) -> ApiError {
    match err {
        ExtError::Denied(m) => ApiError::Forbidden(m),
        ExtError::Request(m) => ApiError::BadRequest(m),
        other => ApiError::Internal(other.to_string()),
    }
}
