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
