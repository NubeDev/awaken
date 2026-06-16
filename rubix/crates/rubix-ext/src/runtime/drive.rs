//! Handler-drives: turn a gated `lifecycle` command into a supervisor action.
//!
//! The transition path (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! "Recommendation: handler-drives"). [`drive_lifecycle`] first crosses the gate
//! via [`crate::lifecycle`] — the capability check
//! ([`ExtensionManage`](rubix_gate::Capability::ExtensionManage)) is fail closed,
//! so an out-of-grant call spawns **nothing**, and the durable `lifecycle` record
//! (with its audit row) lands before any process is touched. Only *after* that
//! write succeeds does it drive the supervisor (`start` spawns, `stop`/`disable`
//! shut down) and report the observed state, so an HTTP caller learns whether the
//! child actually came up rather than racing a reactive watcher.
//!
//! Metrics are folded in here because this is the one place every lifecycle
//! command crosses: a successful command bumps `commands`, a failure bumps
//! `command_errors`, and a fail-closed denial also records a capability
//! violation against the extension's supervisor handle (when one is live) so the
//! metrics view reflects authorization health.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{CorrelationId, Id, Principal};

use crate::control::{ControlRequest, LifecycleAction};
use crate::error::{ExtError, Result};
use crate::supervisor::{ExtensionId, Identity, LifecycleState, ProcessSpec};

use super::ExtensionRuntime;

/// The result of driving a lifecycle command through the gate and the supervisor.
#[derive(Debug, Clone)]
pub struct LifecycleOutcome {
    /// The control record the command acted on.
    pub target: Id,
    /// The correlation id the gate stamped onto the command and audit row.
    pub correlation_id: CorrelationId,
    /// The transition that was requested.
    pub action: LifecycleAction,
    /// The observed supervisor state after driving it. `Some` after a `start`
    /// (the spawned child's state); `None` after a `stop`/`disable` (the
    /// supervisor was torn down, so there is no live handle to report).
    pub state: Option<LifecycleState>,
}

/// Drive a `lifecycle` control request through the gate, then bring the
/// supervisor into agreement with it.
///
/// `spec` is the [`ProcessSpec`] read off the extension's config record (only
/// consulted for a `start`); `identity` is the principal's credentials the child
/// authenticates with (Open question 2 — supplied by the caller, never minted
/// here). On `start` of a non-process flavour the supervisor is not engaged (no
/// child to spawn) and `state` is `None`.
///
/// # Errors
/// Returns [`ExtError::Denied`] if the extension lacks `extension-manage` (before
/// any process is touched), [`ExtError::Request`] on a malformed request, or
/// [`ExtError::Command`] if the gated write or the spawn fails.
pub async fn drive_lifecycle(
    rt: &ExtensionRuntime,
    db: &Surreal<Db>,
    extension: &Principal,
    request: &ControlRequest,
    spec: ProcessSpec,
    identity: Identity,
) -> Result<LifecycleOutcome> {
    let id = ExtensionId::from(extension);
    let counters = rt.metrics.counters(&id);

    let outcome = match crate::lifecycle(db, extension, request).await {
        Ok(outcome) => {
            counters.record_command();
            outcome
        }
        Err(e) => {
            counters.record_command_error();
            // A fail-closed denial is an authorization-health signal: record it
            // against the live handle (if any) so it surfaces on the metrics
            // view alongside process health.
            if matches!(e, ExtError::Denied(_))
                && let Some(handle) = rt.supervisors.get(&id)
            {
                handle.record_violation();
            }
            return Err(e);
        }
    };

    // The gate validated the action, so this re-parse cannot fail.
    let action = request
        .params
        .get("action")
        .and_then(serde_json::Value::as_str)
        .and_then(LifecycleAction::parse)
        .expect("gate accepted the lifecycle action");

    let state = match action {
        LifecycleAction::Start => {
            if spec.flavour.reports_process_stats() {
                let handle = rt.supervisors.start(id, spec, identity)?;
                Some(handle.lifecycle_state())
            } else {
                // Builtin/wasm: the gated record says "enabled", but there is no
                // child to supervise — the host runs the work in-process.
                None
            }
        }
        LifecycleAction::Stop | LifecycleAction::Disable => {
            rt.supervisors.stop(&id).await;
            None
        }
    };

    Ok(LifecycleOutcome {
        target: outcome.target,
        correlation_id: outcome.correlation_id,
        action,
        state,
    })
}
