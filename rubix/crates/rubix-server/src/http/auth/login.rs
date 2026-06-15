//! `POST /auth/login` — exchange a subject + secret for an opaque bearer token.
//!
//! The browser path into the gate: verify the credentials once, then mint a
//! short-lived, revocable login token the UI carries thereafter so the raw secret
//! is not shipped on every request (`rubix/docs/design/BACKEND-COLLECTIONS.md`,
//! "Auth — close the session-issuance gap"). Verification reuses the gate's
//! `authenticate`; the token is scoped to the server's active namespace/database,
//! so it resolves only there.

use axum::Json;
use axum::extract::State;
use rubix_gate::{DEFAULT_TTL_SECONDS, PrincipalToken, authenticate, issue_session_token};

use crate::dto::auth::{LoginRequest, LoginResponse};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Authenticate the credentials and return a fresh login token.
pub async fn login_route(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let credentials = PrincipalToken::new(body.subject, body.secret);

    // Verify before issuing — a token is only minted for a valid credential pair.
    authenticate(state.store.raw(), &credentials)
        .await
        .map_err(|e| ApiError::Unauthenticated(e.to_string()))?;

    let issued = issue_session_token(
        state.store.raw(),
        &credentials,
        &state.namespace,
        &state.database,
        DEFAULT_TTL_SECONDS,
    )
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse {
        token: issued.value,
        expires: issued.expires,
    }))
}
