//! Mint a scoped SurrealDB session for an authenticated principal.
//!
//! This fills the WS-01 store seam (`rubix_store::issue_scoped_session`) with the
//! real identity-aware issuance. A [`ScopedSession`] owns a private clone of the
//! store connection that has been signed in as the principal, so every read it
//! runs is confined by SurrealDB row-level permissions to the principal's
//! namespace (`rubix/STACK-DEISGN.md`, contracts #1/#2). The session is issued
//! once, not per message.

use rubix_core::Principal;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::Result;
use crate::token::PrincipalToken;

use super::scope::sign_in_as_principal;

/// A SurrealDB session scoped to one principal's namespace.
///
/// Holds the principal identity and a connection clone that is authenticated as
/// that principal. Reads run on [`ScopedSession::connection`]; the engine — not
/// the app — limits them to the principal's namespace.
#[derive(Clone, Debug)]
pub struct ScopedSession {
    principal: Principal,
    connection: Surreal<Db>,
}

impl ScopedSession {
    /// The principal this session is scoped to.
    #[must_use]
    pub fn principal(&self) -> &Principal {
        &self.principal
    }

    /// The principal-scoped connection reads run on.
    ///
    /// Cloning the connection elsewhere would inherit this scope, so callers
    /// read through this borrow rather than detaching it.
    #[must_use]
    pub fn connection(&self) -> &Surreal<Db> {
        &self.connection
    }
}

/// Issue a scoped session for `principal` by signing a fresh connection in.
///
/// `root` is the store handle's connection (an owner session). Cloning it yields
/// an independent session over the same datastore; signing that clone in as the
/// principal scopes it without touching `root`. `namespace`/`database` are the
/// SurrealDB namespace/database the principal authenticates against (the
/// principal's own namespace on edge; the tenant namespace on cloud).
///
/// # Errors
/// Returns [`GateError::IssueSession`](crate::GateError::IssueSession) if the
/// principal's credentials are rejected by the access method.
pub async fn issue_scoped_session(
    root: &Surreal<Db>,
    namespace: &str,
    database: &str,
    principal: Principal,
    token: &PrincipalToken,
) -> Result<ScopedSession> {
    let connection = root.clone();
    sign_in_as_principal(&connection, namespace, database, token).await?;
    Ok(ScopedSession {
        principal,
        connection,
    })
}
