//! The transport error type and its HTTP response mapping.
//!
//! Every route returns `Result<_, ApiError>`; this maps a failure to an HTTP
//! status and a JSON body so handlers stay thin (`rubix/docs/sessions/WS-16.md`:
//! extract → call domain → map DTO → return). The gate's fail-closed denials
//! surface as `403`, an absent record as `404`, a malformed credential as `401`,
//! and an unexpected store/engine failure as `500` — the wire never leaks a raw
//! engine string as a success.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

/// A transport-layer error mapped to an HTTP response.
#[derive(Debug, Error)]
pub enum ApiError {
    /// The request carried no or malformed principal credentials.
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),
    /// The principal is authenticated but lacks the capability for the action.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// The addressed resource does not exist or is not visible to the principal.
    #[error("not found")]
    NotFound,
    /// The request body or parameters were invalid.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// The request conflicts with existing state (e.g. a duplicate datasource id).
    #[error("conflict: {0}")]
    Conflict(String),
    /// An upstream the request depends on (e.g. an external datasource) is
    /// unreachable. The request was well-formed; the dependency is the problem.
    #[error("bad gateway: {0}")]
    BadGateway(String),
    /// An unexpected store, gate, or engine failure.
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    /// The HTTP status this error maps to.
    fn status(&self) -> StatusCode {
        match self {
            ApiError::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::BadGateway(_) => StatusCode::BAD_GATEWAY,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}

/// Result alias for transport handlers.
pub type ApiResult<T> = std::result::Result<T, ApiError>;
