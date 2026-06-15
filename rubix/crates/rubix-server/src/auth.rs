//! Resolve the request's principal and its scoped read session.
//!
//! Every authenticated route needs two things from the gate: the [`Principal`]
//! (for a mutation [`Command`](rubix_gate::Command)) and the WS-03 scoped session
//! (for a read). This extractor reads the credential header pair, resolves it
//! through `rubix_gate::authenticate`, and issues the scoped session once — the
//! single auth path for users and extensions (`rubix/docs/SCOPE.md`, principle 5).
//!
//! The wire credential is the principal's subject + secret as headers, the same
//! pair the gate's record access method verifies natively
//! (`rubix-gate::PrincipalToken`); no JWT/session-cookie layer is introduced here
//! (see the WS-16 session log "Assumptions").

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rubix_core::Principal;
use rubix_gate::{PrincipalToken, ScopedSession, authenticate, issue_scoped_session};

use crate::error::ApiError;
use crate::state::AppState;

/// The credential header carrying the principal's subject.
const SUBJECT_HEADER: &str = "x-rubix-subject";
/// The credential header carrying the principal's secret.
const SECRET_HEADER: &str = "x-rubix-secret";

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
        let subject = header(parts, SUBJECT_HEADER)?;
        let secret = header(parts, SECRET_HEADER)?;
        let token = PrincipalToken::new(subject, secret);

        let principal = authenticate(state.store.raw(), &token)
            .await
            .map_err(|e| ApiError::Unauthenticated(e.to_string()))?;
        let session = issue_scoped_session(
            state.store.raw(),
            &state.namespace,
            &state.database,
            principal.clone(),
            &token,
        )
        .await
        .map_err(|e| ApiError::Unauthenticated(e.to_string()))?;

        Ok(Self { principal, session })
    }
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
