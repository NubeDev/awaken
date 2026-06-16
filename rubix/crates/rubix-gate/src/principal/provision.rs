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

/// Provision or rotate `principal`'s identity, overwriting any existing secret.
///
/// Unlike [`provision_principal`] — which `create`s the identity once and errors
/// if the subject is already taken — this upserts. It is the path a long-lived
/// **system actor** uses to re-establish its own login on each boot: a background
/// worker (e.g. the hook dispatcher) cannot recover a previously stored secret
/// (the access method keeps only what `SIGNIN` checks), so it rotates to a fresh
/// secret it holds in memory for the process lifetime. A restart simply rotates
/// again — there is no plaintext secret to persist or leak. Provisioning stays an
/// owner action on the root handle.
///
/// # Errors
/// Returns [`GateError::IssueSession`] if the identity write fails.
pub async fn reprovision_principal(
    db: &Surreal<Db>,
    principal: &Principal,
    secret: impl Into<String>,
) -> Result<()> {
    let row = PrincipalRow::new(principal, secret);
    let _: Option<PrincipalRow> = db
        .upsert((PRINCIPAL_TABLE, principal.subject.as_str()))
        .content(row)
        .await
        .map_err(GateError::IssueSession)?;
    Ok(())
}
