//! `POST /auth/logout` — revoke the presented bearer login token.
//!
//! Revocation is what makes an opaque server-side token better than a bare JWT:
//! deleting the token row invalidates it immediately, with no "valid until expiry
//! regardless" window (`rubix/docs/design/BACKEND-COLLECTIONS.md`, OQ10). Logout
//! is idempotent — an unknown or already-revoked token is not an error.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use rubix_gate::revoke_session_token;

use crate::auth::bearer_token;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Revoke the bearer token carried on the request, if any.
///
/// Returns `204 No Content` whether or not a token was actually deleted (a
/// double logout is a no-op), and `400` only when no bearer token was presented.
pub async fn logout_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let token = bearer_token(&headers)
        .ok_or_else(|| ApiError::BadRequest("no bearer token to revoke".to_owned()))?;
    revoke_session_token(state.store.raw(), &token)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
