//! JSON-RPC `register` → command via the WS-05 gate.
//!
//! `register` creates the extension's configuration record (`rubix/docs/sessions/
//! WS-13.md`). It is a *control-plane* command — distinct from provisioning the
//! extension's identity ([`register_extension`](crate::register_extension), the
//! WS-03 owner write): this method is the extension declaring its own
//! configuration through the gate, so it is capability-checked
//! ([`DatasourceRegister`](rubix_gate::Capability::DatasourceRegister)) and
//! audited like any command (contract #1). Routed as a
//! [`Create`](rubix_gate::Change::Create) so the configuration record lands in
//! the extension's namespace with a before/after capture.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_gate::{Change, Command, apply};

use crate::error::{ExtError, Result};

use super::authorize::authorize;
use super::request::{ControlMethod, ControlOutcome, ControlRequest};

/// Drive a `register` control request through the gate as `extension`.
///
/// Authorizes the `DatasourceRegister` capability fail closed, then creates the
/// extension's configuration record from the request params. Returns the
/// correlation id the gate stamped onto the command and audit row.
///
/// # Errors
/// Returns [`ExtError::Denied`] if the extension lacks the grant (before any
/// write), [`ExtError::Request`] if the request is not a `register`, or
/// [`ExtError::Command`] if the gated mutation fails.
pub async fn register(
    db: &Surreal<Db>,
    extension: &Principal,
    request: &ControlRequest,
) -> Result<ControlOutcome> {
    if request.method != ControlMethod::Register {
        return Err(ExtError::Request(format!(
            "expected register, got {}",
            request.method.as_str()
        )));
    }
    let capability = ControlMethod::Register
        .required_capability()
        .expect("register is a gated command");
    authorize(db, extension, capability).await?;

    let command = Command::new(
        extension.clone(),
        capability,
        request.target.clone(),
        Change::Create(request.params.clone()),
    );
    let applied = apply(db, &command, None)
        .await
        .map_err(|e| ExtError::Command(e.to_string()))?;
    Ok(ControlOutcome {
        target: request.target.clone(),
        correlation_id: applied.correlation_id,
    })
}
