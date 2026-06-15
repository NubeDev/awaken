//! JSON-RPC `configure` → command via the WS-05 gate.
//!
//! `configure` updates the extension's configuration record (`rubix/docs/
//! sessions/WS-13.md`). Like every extension command it is capability-checked
//! ([`DatasourceRegister`](rubix_gate::Capability::DatasourceRegister)) and
//! audited (contract #1), routed as an [`Update`](rubix_gate::Change::Update) so
//! the prior configuration is captured before/after atomically with the write —
//! a reconfiguration leaves an audit trail of what changed.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_gate::{Change, Command, apply};

use crate::error::{ExtError, Result};

use super::authorize::authorize;
use super::request::{ControlMethod, ControlOutcome, ControlRequest};

/// Drive a `configure` control request through the gate as `extension`.
///
/// Authorizes the `DatasourceRegister` capability fail closed, then updates the
/// extension's configuration record from the request params. Returns the
/// correlation id the gate stamped onto the command and audit row.
///
/// # Errors
/// Returns [`ExtError::Denied`] if the extension lacks the grant (before any
/// write), [`ExtError::Request`] if the request is not a `configure`, or
/// [`ExtError::Command`] if the gated mutation fails.
pub async fn configure(
    db: &Surreal<Db>,
    extension: &Principal,
    request: &ControlRequest,
) -> Result<ControlOutcome> {
    if request.method != ControlMethod::Configure {
        return Err(ExtError::Request(format!(
            "expected configure, got {}",
            request.method.as_str()
        )));
    }
    let capability = ControlMethod::Configure
        .required_capability()
        .expect("configure is a gated command");
    authorize(db, extension, capability).await?;

    let command = Command::new(
        extension.clone(),
        capability,
        request.target.clone(),
        Change::Update(request.params.clone()),
    );
    let applied = apply(db, &command, None)
        .await
        .map_err(|e| ExtError::Command(e.to_string()))?;
    Ok(ControlOutcome {
        target: request.target.clone(),
        correlation_id: applied.correlation_id,
    })
}
