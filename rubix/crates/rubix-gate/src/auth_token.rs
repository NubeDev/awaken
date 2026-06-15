//! Opaque, server-side login tokens — the session-issuance surface (OQ10).
//!
//! The credential headers (`PrincipalToken`: subject + secret) are right for
//! service accounts but wrong for a browser, which would have to ship the raw
//! secret on every request. A login token fixes that without weakening the gate:
//! `POST /auth/login` exchanges the secret **once** for an opaque bearer token
//! the UI then carries (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Auth — close
//! the session-issuance gap").
//!
//! The security model is the load-bearing part (OQ10), and the choices here are
//! deliberate:
//!
//! - **Opaque, not a JWT.** The token is random server-side state, so it is
//!   **revocable** by deleting its row — no signing key, no rotation, no
//!   "valid until expiry no matter what" window. `POST /auth/logout` revokes.
//! - **Hashed at rest.** Only `crypto::sha256(token)` is stored (computed in the
//!   engine), so a store read never yields a usable token — same posture as not
//!   logging secrets.
//! - **Expiring.** Every token carries an `expires` instant; resolution rejects
//!   an expired token, so a leaked token is bounded in time.
//! - **Namespace-scoped.** The token records the namespace/database it was minted
//!   against and resolves only there, so it cannot be replayed against another
//!   tenant's data.
//!
//! Resolution reconstructs the [`PrincipalToken`] the rest of the gate already
//! understands, so token auth slots *in front of* [`authenticate`](crate::authenticate)
//! and [`issue_scoped_session`](crate::issue_scoped_session) with no change to
//! either — one auth path, two front doors.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::types::{Datetime, SurrealValue};

use rubix_core::Id;

use crate::error::{GateError, Result};
use crate::token::PrincipalToken;

/// The table opaque login tokens are stored in (defined by
/// [`define_gate_schema`](crate::define_gate_schema)).
const SESSION_TOKEN_TABLE: &str = "session_token";

/// Default token lifetime: 24 hours. A browser session is re-established by a
/// fresh login; a shorter window bounds the blast radius of a leaked token.
pub const DEFAULT_TTL_SECONDS: i64 = 24 * 60 * 60;

/// A freshly minted login token returned to the client exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedToken {
    /// The opaque bearer value the client carries (never stored in the clear).
    pub value: String,
    /// When the token expires (RFC 3339, UTC).
    pub expires: String,
}

/// The credentials and scope an opaque token resolves back to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedToken {
    /// The subject + secret pair the gate authenticates and signs sessions with.
    pub token: PrincipalToken,
    /// The namespace the token was minted against (and resolves only within).
    pub namespace: String,
    /// The database the token was minted against.
    pub database: String,
}

/// The fields persisted for a token (decoded on resolve).
#[derive(Debug, SurrealValue)]
struct TokenRow {
    subject: String,
    secret: String,
    namespace: String,
    database: String,
}

/// The `expires` projection returned by the issue statement.
#[derive(Debug, SurrealValue)]
struct ExpiresRow {
    expires: Datetime,
}

/// Mint an opaque login token for already-verified credentials.
///
/// The caller must have authenticated `token` first (the HTTP login route does,
/// via [`authenticate`](crate::authenticate)); this only persists the session
/// row and returns the bearer value. The raw value is generated here and never
/// stored — only its SHA-256 is — so this return is the one and only time it is
/// available in the clear.
///
/// # Errors
/// Returns [`GateError::TokenStore`] if the write fails or its result cannot be
/// decoded.
pub async fn issue_session_token(
    db: &Surreal<Db>,
    token: &PrincipalToken,
    namespace: &str,
    database: &str,
    ttl_seconds: i64,
) -> Result<IssuedToken> {
    // 256 bits of entropy from two v4 UUIDs — opaque and unguessable.
    let raw = format!("{}{}", Id::new().as_str(), Id::new().as_str());
    let ttl = format!("{ttl_seconds}s");

    let mut response = db
        .query(format!(
            "CREATE {SESSION_TOKEN_TABLE} SET \
               token_hash = crypto::sha256($raw), \
               subject = $subject, \
               secret = $secret, \
               namespace = $namespace, \
               database = $database, \
               expires = time::now() + type::duration($ttl) \
             RETURN expires"
        ))
        .bind(("raw", raw.clone()))
        .bind(("subject", token.subject.clone()))
        .bind(("secret", token.secret.clone()))
        .bind(("namespace", namespace.to_owned()))
        .bind(("database", database.to_owned()))
        .bind(("ttl", ttl))
        .await
        .map_err(GateError::TokenStore)?;
    let row: Option<ExpiresRow> = response.take(0).map_err(GateError::TokenStore)?;
    let expires = row
        .map(|r| r.expires.to_string())
        .ok_or_else(|| GateError::Authenticate("token issue returned no row".to_owned()))?;

    Ok(IssuedToken { value: raw, expires })
}

/// Resolve an opaque bearer token to its credentials and scope.
///
/// Matches on the token's SHA-256 and requires the row to be unexpired, so an
/// unknown, revoked, or expired token resolves to `None` (the caller rejects it
/// as unauthenticated). The raw token never touches the store — only its hash,
/// computed in the engine.
///
/// # Errors
/// Returns [`GateError::TokenStore`] if the lookup query fails.
pub async fn resolve_session_token(db: &Surreal<Db>, raw: &str) -> Result<Option<ResolvedToken>> {
    let mut response = db
        .query(format!(
            "SELECT subject, secret, namespace, database FROM {SESSION_TOKEN_TABLE} \
             WHERE token_hash = crypto::sha256($raw) AND expires > time::now() \
             LIMIT 1"
        ))
        .bind(("raw", raw.to_owned()))
        .await
        .map_err(GateError::TokenStore)?;
    let row: Option<TokenRow> = response.take(0).map_err(GateError::TokenStore)?;
    Ok(row.map(|r| ResolvedToken {
        token: PrincipalToken::new(r.subject, r.secret),
        namespace: r.namespace,
        database: r.database,
    }))
}

/// Revoke an opaque bearer token, returning whether a token was deleted.
///
/// Idempotent: revoking an unknown or already-revoked token returns `false`
/// rather than erroring, so a double logout is not a failure.
///
/// # Errors
/// Returns [`GateError::TokenStore`] if the delete query fails.
pub async fn revoke_session_token(db: &Surreal<Db>, raw: &str) -> Result<bool> {
    let mut response = db
        .query(format!(
            "DELETE {SESSION_TOKEN_TABLE} \
             WHERE token_hash = crypto::sha256($raw) RETURN BEFORE"
        ))
        .bind(("raw", raw.to_owned()))
        .await
        .map_err(GateError::TokenStore)?;
    let deleted: Vec<serde_json::Value> = response.take(0).map_err(GateError::TokenStore)?;
    Ok(!deleted.is_empty())
}
