//! JSON-RPC `invoke` → capability check → command through the WS-05 gate.
//!
//! `invoke` is the extension's gated, audited effect (`rubix/docs/sessions/
//! WS-13.md`): it first confirms the extension holds the
//! [`RuleInvoke`](rubix_gate::Capability::RuleInvoke) grant
//! ([`authorize`](super::authorize)), fail closed before anything happens, then
//! routes the invocation as a [`Command`](rubix_gate::Command) through
//! [`apply`](rubix_gate::apply). The gate re-checks the grant, mints the
//! correlation id, captures before/after atomically, and appends the immutable
//! audit row — so a granted `invoke` writes exactly one audit row carrying its
//! correlation id, and an out-of-grant `invoke` is denied before any write
//! (contracts #1, #2). Each invocation is recorded as a *fresh* generic record
//! holding the invocation params — the same shape WS-11 records a rule insight
//! (`rubix-rules`) — so an invoke is a single gated, append-only event, not an
//! un-audited side channel.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_gate::{Change, Command, apply};

use crate::error::{ExtError, Result};

use super::authorize::authorize;
use super::request::{ControlMethod, ControlOutcome, ControlRequest};

/// Drive an `invoke` control request through the gate as `extension`.
///
/// Authorizes the `RuleInvoke` capability fail closed, then creates a fresh
/// record at the request's target carrying the invocation params. Returns the
/// [`ControlOutcome`] with the correlation id the gate stamped onto the command
/// and audit row.
///
/// # Errors
/// Returns [`ExtError::Denied`] if the extension lacks the grant (before any
/// write), [`ExtError::Request`] if the request is not an `invoke`, or
/// [`ExtError::Command`] if the gated mutation fails.
pub async fn invoke(
    db: &Surreal<Db>,
    extension: &Principal,
    request: &ControlRequest,
) -> Result<ControlOutcome> {
    if request.method != ControlMethod::Invoke {
        return Err(ExtError::Request(format!(
            "expected invoke, got {}",
            request.method.as_str()
        )));
    }
    let capability = ControlMethod::Invoke
        .required_capability()
        .expect("invoke is a gated command");
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
