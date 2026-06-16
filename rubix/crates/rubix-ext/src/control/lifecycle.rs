//! JSON-RPC `lifecycle` (start / stop / disable) → command via the WS-05 gate.
//!
//! `lifecycle` transitions the extension's run state (`rubix/docs/sessions/
//! WS-13.md`). Each transition is a gated, audited command: it is
//! capability-checked ([`DatasourceRegister`](rubix_gate::Capability::DatasourceRegister))
//! and routed as an [`Update`](rubix_gate::Change::Update) writing the requested
//! [`LifecycleAction`] onto the extension's control record, so every start/stop/
//! disable leaves an audit row (contract #1). A `disable` is audited like any
//! other transition — there is no un-gated off switch.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_gate::{Change, Command, apply};

use crate::authz::authorize;
use crate::error::{ExtError, Result};

use super::request::{ControlMethod, ControlOutcome, ControlRequest};

/// A lifecycle transition an extension can be driven through.
///
/// The set is closed; the stable wire string is from [`LifecycleAction::as_str`].
/// An unknown action string resolves to `None` so the method fails closed rather
/// than guessing a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleAction {
    /// Bring the extension into its running state.
    Start,
    /// Halt the extension without disabling it.
    Stop,
    /// Disable the extension (a halted, non-restartable state until re-enabled).
    Disable,
}

impl LifecycleAction {
    /// The stable wire string for this action.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleAction::Start => "start",
            LifecycleAction::Stop => "stop",
            LifecycleAction::Disable => "disable",
        }
    }

    /// Resolve a wire string to a known action, or `None` to fail closed.
    #[must_use]
    pub fn parse(raw: &str) -> Option<LifecycleAction> {
        match raw {
            "start" => Some(LifecycleAction::Start),
            "stop" => Some(LifecycleAction::Stop),
            "disable" => Some(LifecycleAction::Disable),
            _ => None,
        }
    }
}

/// Drive a `lifecycle` control request through the gate as `extension`.
///
/// Reads the requested transition from the request params' `action` field
/// (fail closed on an unknown action), authorizes the capability, then writes the
/// new lifecycle state onto the extension's control record through the gate.
/// Returns the correlation id the gate stamped onto the command and audit row.
///
/// # Errors
/// Returns [`ExtError::Request`] if the request is not a `lifecycle` or carries
/// no recognised `action`, [`ExtError::Denied`] if the extension lacks the grant
/// (before any write), or [`ExtError::Command`] if the gated mutation fails.
pub async fn lifecycle(
    db: &Surreal<Db>,
    extension: &Principal,
    request: &ControlRequest,
) -> Result<ControlOutcome> {
    if request.method != ControlMethod::Lifecycle {
        return Err(ExtError::Request(format!(
            "expected lifecycle, got {}",
            request.method.as_str()
        )));
    }
    let action = request
        .params
        .get("action")
        .and_then(serde_json::Value::as_str)
        .and_then(LifecycleAction::parse)
        .ok_or_else(|| {
            ExtError::Request("lifecycle requires a known `action` param".to_owned())
        })?;
    let capability = ControlMethod::Lifecycle
        .required_capability()
        .expect("lifecycle is a gated command");
    authorize(db, extension, capability).await?;

    let command = Command::new(
        extension.clone(),
        capability,
        request.target.clone(),
        Change::Update(serde_json::json!({ "lifecycle": action.as_str() })),
    );
    let applied = apply(db, &command, None)
        .await
        .map_err(|e| ExtError::Command(e.to_string()))?;
    Ok(ControlOutcome {
        target: request.target.clone(),
        correlation_id: applied.correlation_id,
    })
}

#[cfg(test)]
mod tests {
    use super::LifecycleAction;

    #[test]
    fn lifecycle_actions_round_trip_through_their_strings() {
        for action in [
            LifecycleAction::Start,
            LifecycleAction::Stop,
            LifecycleAction::Disable,
        ] {
            assert_eq!(LifecycleAction::parse(action.as_str()), Some(action));
        }
        assert_eq!(LifecycleAction::parse("teleport"), None);
    }
}
