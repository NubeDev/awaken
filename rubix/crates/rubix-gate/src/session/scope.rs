//! Bind a fresh SurrealDB session to a principal by signing it in.
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`): a scoped read session is gate-issued,
//! not a per-message proxy. The mechanism: clone the root handle's
//! `Surreal<Db>` — that yields a new session id over the *same* datastore — then
//! `signin` that clone through the `principal` record access method. Signing in
//! binds `$auth` to the principal record on this session only; the root session
//! is untouched. Row-level permissions on the `record` table then enforce reads
//! natively for everything this session does.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::opt::auth::Record;
use surrealdb::types::SurrealValue;

use crate::error::{GateError, Result};
use crate::token::PrincipalToken;

/// The record access method defined by [`define_gate_schema`](crate::define_gate_schema).
const PRINCIPAL_ACCESS: &str = "principal";

/// The variables the access method's `SIGNIN` query binds (`$subject`,
/// `$secret`).
#[derive(Debug, SurrealValue)]
struct SigninParams {
    subject: String,
    secret: String,
}

/// Sign `session` in as the principal identified by `token`, scoped to
/// `namespace`/`database`.
///
/// `session` must be a clone of the root handle's connection so the signin
/// affects only this session. After this returns, the engine treats the session
/// as the principal and enforces the `record` table's row-level read permission.
///
/// # Errors
/// Returns [`GateError::IssueSession`] if the credentials are rejected by the
/// access method's `SIGNIN` query (unknown subject or wrong secret).
pub(crate) async fn sign_in_as_principal(
    session: &Surreal<Db>,
    namespace: &str,
    database: &str,
    token: &PrincipalToken,
) -> Result<()> {
    session
        .signin(Record {
            namespace: namespace.to_owned(),
            database: database.to_owned(),
            access: PRINCIPAL_ACCESS.to_owned(),
            params: SigninParams {
                subject: token.subject.clone(),
                secret: token.secret.clone(),
            },
        })
        .await
        .map_err(GateError::IssueSession)?;
    Ok(())
}
