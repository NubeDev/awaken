use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Unavailable(&'static str),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

/// A `his` hot→cold flush failure: a store read/delete or a Parquet write.
#[derive(Debug, thiserror::Error)]
pub enum FlushError {
    #[error(transparent)]
    Store(#[from] crate::store::StoreError),
    #[error("parquet cold tier: {0}")]
    Tier(#[from] rubix_query::QueryError),
}

impl From<FlushError> for ApiError {
    fn from(err: FlushError) -> Self {
        match err {
            FlushError::Store(e) => e.into(),
            FlushError::Tier(e) => ApiError::Internal(anyhow::anyhow!(e)),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: String,
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let ApiError::Internal(err) = &self {
            tracing::error!(error = ?err, "internal error");
        }
        let body = ErrorBody {
            error: self.to_string(),
        };
        (self.status(), Json(body)).into_response()
    }
}

impl From<rubix_core::CoreError> for ApiError {
    fn from(err: rubix_core::CoreError) -> Self {
        ApiError::BadRequest(err.to_string())
    }
}

impl From<rubix_datasource::DatasourceError> for ApiError {
    fn from(err: rubix_datasource::DatasourceError) -> Self {
        use rubix_datasource::DatasourceError as E;
        match err {
            // An unknown id or named query is a missing resource, not a server
            // fault. (An unscoped deployment is global by design — see
            // docs/design/datasources.md "Tenancy" — so existence is not secret.)
            E::UnknownDatasource(_) => ApiError::NotFound("datasource"),
            E::UnknownQuery { .. } => ApiError::NotFound("named query"),
            // Caller-shaped failures: bad SQL, wrong arity, or a breach the
            // caller asked to treat strictly. All 400s the caller can correct.
            E::MultiStatement
            | E::EmptyStatement
            | E::ParamCount { .. }
            | E::CapBreached { .. }
            | E::Manifest(_) => ApiError::BadRequest(err.to_string()),
            // Connection / backend-read failures are server-side. The error
            // carries no credentials (the registry never leaks the password),
            // so the message is safe to log; the response is a generic 500.
            E::Connect { .. } | E::Backend { .. } => ApiError::Internal(anyhow::anyhow!(err)),
        }
    }
}

impl From<crate::auth::AuthError> for ApiError {
    fn from(err: crate::auth::AuthError) -> Self {
        use crate::auth::AuthError;
        match err {
            AuthError::MissingToken | AuthError::InvalidToken(_) => {
                ApiError::Unauthorized(err.to_string())
            }
            AuthError::Forbidden(msg) => ApiError::Forbidden(msg),
            AuthError::Misconfigured(msg) => ApiError::Internal(anyhow::anyhow!(msg)),
        }
    }
}
