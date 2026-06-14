//! Resolve a token to a [`Principal`](rubix_core::Principal).
//!
//! Authentication is the gate's entry point: a [`PrincipalToken`] (subject +
//! secret) is verified against the persisted `principal` record on the root
//! handle. A valid token yields the principal identity (without the secret); an
//! unknown subject or a wrong secret is rejected. The same path authenticates a
//! user and an extension — one identity model (`rubix/docs/SCOPE.md`, principle
//! 5).
//!
//! This resolves *who* the principal is. Minting the scoped SurrealDB session
//! that *enforces* what it may read is [`issue_scoped_session`](crate::issue_scoped_session).

use rubix_core::Principal;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{GateError, Result};
use crate::principal::row::{PRINCIPAL_TABLE, PrincipalRow};
use crate::token::PrincipalToken;

/// Verify `token` against the persisted principal and return its identity.
///
/// Runs on the root handle so it can read the secret column the access method's
/// `SIGNIN` query also checks. The returned [`Principal`] never carries the
/// secret.
///
/// # Errors
/// Returns [`GateError::Authenticate`] if the subject is unknown, the secret
/// does not match, or the stored row has an unrecognised kind/role. Returns
/// [`GateError::Lookup`] if the lookup query itself fails.
pub async fn authenticate(db: &Surreal<Db>, token: &PrincipalToken) -> Result<Principal> {
    let row: Option<PrincipalRow> = db
        .select((PRINCIPAL_TABLE, token.subject.as_str()))
        .await
        .map_err(GateError::Lookup)?;
    let row = row.ok_or_else(|| GateError::Authenticate("unknown principal".to_owned()))?;
    if row.secret != token.secret {
        return Err(GateError::Authenticate("invalid secret".to_owned()));
    }
    row.into_principal()
        .ok_or_else(|| GateError::Authenticate("principal record is malformed".to_owned()))
}
