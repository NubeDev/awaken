//! Authorize a command against the principal's capability grant (WS-04).
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`): the gate checks the capability grant
//! *before* applying any change. This step is the command path's call into the
//! app-enforced authz layer ([`check_capability`](crate::check_capability)): a
//! principal that does not hold the command's required capability is refused
//! here, so no write — and no audit row — is ever produced for a denied command
//! (fail closed, `rubix/docs/SCOPE.md`, "Two authz layers").

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::capability::check_capability;
use crate::error::{GateError, Result};

use super::define::Command;

/// Refuse `command` unless its principal holds the required capability grant.
///
/// Returns `Ok(())` only when the grant exists. A missing grant or an unknown
/// capability becomes [`GateError::CommandDenied`]; the decision is taken before
/// the capture/apply step runs, so a denied command never reaches the store.
///
/// # Errors
/// Returns [`GateError::CommandDenied`] if the principal lacks the grant, or
/// [`GateError::GrantStore`] if the grant lookup itself fails.
pub(crate) async fn authorize(db: &Surreal<Db>, command: &Command) -> Result<()> {
    let allowed = check_capability(db, &command.principal, command.capability).await?;
    if allowed {
        return Ok(());
    }
    Err(GateError::CommandDenied(format!(
        "{} lacks capability {} in namespace {}",
        command.principal.subject,
        command.capability.as_str(),
        command.namespace()
    )))
}
