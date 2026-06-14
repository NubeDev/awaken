//! Auth failures and their HTTP mapping. Authentication failures (missing or
//! invalid bearer) are `401`; authorization failures (valid principal, out of
//! scope) are `403`. Both fail closed — a verifier that cannot reach its JWKS
//! rejects rather than admits.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error::ErrorBody;

/// Why a request was not authenticated or not authorized.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// No bearer token was presented where one is required.
    #[error("missing bearer token")]
    MissingToken,
    /// The bearer token failed validation (bad signature, expired, wrong
    /// issuer, unknown PAT, malformed claims).
    #[error("invalid token: {0}")]
    InvalidToken(String),
    /// The token validated but the principal's scope does not cover the
    /// requested resource, or its role may not perform the action.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// Auth is misconfigured for this profile (e.g. cloud requires an issuer but
    /// none is set). Surfaces as a server-side error, never as an open door.
    #[error("auth misconfigured: {0}")]
    Misconfigured(String),
}

impl AuthError {
    fn status(&self) -> StatusCode {
        match self {
            AuthError::MissingToken | AuthError::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            AuthError::Forbidden(_) => StatusCode::FORBIDDEN,
            AuthError::Misconfigured(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        if let AuthError::Misconfigured(msg) = &self {
            tracing::error!(error = %msg, "auth misconfigured");
        }
        let body = ErrorBody {
            error: self.to_string(),
        };
        (self.status(), Json(body)).into_response()
    }
}
