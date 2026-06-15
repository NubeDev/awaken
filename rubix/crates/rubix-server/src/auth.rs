//! Resolve the request's principal and its scoped read session.
//!
//! Every authenticated route needs two things from the gate: the [`Principal`]
//! (for a mutation [`Command`](rubix_gate::Command)) and the WS-03 scoped session
//! (for a read). This extractor reads the credential header pair, resolves it
//! through `rubix_gate::authenticate`, and issues the scoped session once — the
//! single auth path for users and extensions (`rubix/docs/SCOPE.md`, principle 5).
//!
//! A request authenticates one of two ways, resolved here to the same identity:
//!
//! - an **opaque login token** as `Authorization: Bearer <token>` — the browser
//!   path, exchanged once at `POST /auth/login` so the raw secret is not shipped
//!   per request. The token resolves to the credentials and namespace it was
//!   minted against (`rubix_gate::resolve_session_token`); or
//! - the principal's **subject + secret** as headers — the service-account path,
//!   the same pair the gate's record access method verifies natively
//!   (`rubix-gate::PrincipalToken`).
//!
//! Either way the result is a [`PrincipalToken`] plus the namespace/database to
//! sign into, fed through the unchanged `authenticate` + `issue_scoped_session`
//! path — one auth path, two front doors.

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rubix_core::Principal;
use rubix_gate::{
    PrincipalToken, ScopedSession, authenticate, issue_scoped_session, resolve_session_token,
};

use crate::error::ApiError;
use crate::state::AppState;

/// The credential header carrying the principal's subject.
const SUBJECT_HEADER: &str = "x-rubix-subject";
/// The credential header carrying the principal's secret.
const SECRET_HEADER: &str = "x-rubix-secret";
/// The standard bearer-token authorization header.
const AUTHORIZATION_HEADER: &str = "authorization";
/// The bearer scheme prefix (case-insensitive per RFC 6750, matched lowercased).
const BEARER_PREFIX: &str = "bearer ";

/// An authenticated principal together with its gate-issued scoped session.
///
/// Routes take this as an extractor: a mutation drives a command for
/// [`principal`](Authenticated::principal) through the gate; a read runs on
/// [`session`](Authenticated::session). Both are produced by one authentication,
/// so a handler never re-authenticates per operation.
pub struct Authenticated {
    /// The resolved principal — the subject of authz and audit.
    pub principal: Principal,
    /// The principal's scoped read session.
    pub session: ScopedSession,
}

#[async_trait]
impl FromRequestParts<AppState> for Authenticated {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Credentials {
            token,
            namespace,
            database,
        } = resolve_credentials(parts, state).await?;

        let principal = authenticate(state.store.raw(), &token)
            .await
            .map_err(|e| ApiError::Unauthenticated(e.to_string()))?;
        let session = issue_scoped_session(
            state.store.raw(),
            &namespace,
            &database,
            principal.clone(),
            &token,
        )
        .await
        .map_err(|e| ApiError::Unauthenticated(e.to_string()))?;

        Ok(Self { principal, session })
    }
}

/// The credentials and scope a request resolved to, before authentication.
struct Credentials {
    token: PrincipalToken,
    namespace: String,
    database: String,
}

/// Resolve a request's credentials from a bearer login token or the subject +
/// secret header pair.
///
/// A bearer token is preferred when present; it carries its own namespace/
/// database (so it cannot be replayed against another tenant). The header path
/// falls back to the server's active namespace/database.
async fn resolve_credentials(parts: &Parts, state: &AppState) -> Result<Credentials, ApiError> {
    if let Some(bearer) = bearer_token(&parts.headers) {
        let resolved = resolve_session_token(state.store.raw(), &bearer)
            .await
            .map_err(|e| ApiError::Unauthenticated(e.to_string()))?
            .ok_or_else(|| {
                ApiError::Unauthenticated("invalid or expired login token".to_owned())
            })?;
        return Ok(Credentials {
            token: resolved.token,
            namespace: resolved.namespace,
            database: resolved.database,
        });
    }

    let subject = header(parts, SUBJECT_HEADER)?;
    let secret = header(parts, SECRET_HEADER)?;
    Ok(Credentials {
        token: PrincipalToken::new(subject, secret),
        namespace: state.namespace.clone(),
        database: state.database.clone(),
    })
}

/// Extract the bearer token value from the `Authorization` header, if any.
///
/// Returns `None` when the header is absent or not a `Bearer` credential, so the
/// caller falls through to the subject/secret path. Shared with the logout route,
/// which revokes the presented token.
pub(crate) fn bearer_token(headers: &axum::http::HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION_HEADER)?.to_str().ok()?;
    let rest = value
        .get(..BEARER_PREFIX.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(BEARER_PREFIX))
        .map(|_| &value[BEARER_PREFIX.len()..])?;
    let trimmed = rest.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// Read a required string header, or reject as unauthenticated.
fn header(parts: &Parts, name: &str) -> Result<String, ApiError> {
    parts
        .headers
        .get(name)
        .ok_or_else(|| ApiError::Unauthenticated(format!("missing {name} header")))?
        .to_str()
        .map(str::to_owned)
        .map_err(|_| ApiError::Unauthenticated(format!("{name} header is not valid text")))
}
