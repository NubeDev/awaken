//! Persist a principal identity so it can authenticate and sign in.
//!
//! Provisioning is an owner action: it writes to the `principal` table through
//! the root store handle, binding the identity to the secret the record access
//! method's `SIGNIN` query checks. Users and extensions are provisioned the same
//! way (`rubix/docs/SCOPE.md`, principle 5) — the only difference is the
//! principal's `kind`.

use rubix_core::Principal;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{GateError, Result};

use super::row::{PRINCIPAL_TABLE, PrincipalRow};

/// Provision `principal` with `secret`, keyed by the principal's subject.
///
/// Runs on the root handle (owner session) because writing identity records is
/// a privileged action that precedes any scoped session. The secret is what the
/// principal later presents in its [`PrincipalToken`](crate::PrincipalToken).
///
/// # Errors
/// Returns [`GateError::IssueSession`] if the identity write fails.
pub async fn provision_principal(
    db: &Surreal<Db>,
    principal: &Principal,
    secret: impl Into<String>,
) -> Result<()> {
    let row = PrincipalRow::new(principal, secret);
    let _: Option<PrincipalRow> = db
        .create((PRINCIPAL_TABLE, principal.subject.as_str()))
        .content(row)
        .await
        .map_err(GateError::IssueSession)?;
    Ok(())
}
