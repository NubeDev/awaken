use crate::error::ApiError;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Db(anyhow::Error),
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotFound(what) => ApiError::NotFound(what),
            StoreError::Conflict(msg) => ApiError::Conflict(msg),
            StoreError::Invalid(msg) => ApiError::BadRequest(msg),
            StoreError::Db(err) => ApiError::Internal(err),
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        if let rusqlite::Error::SqliteFailure(code, ref msg) = err {
            if code.code == rusqlite::ErrorCode::ConstraintViolation {
                return StoreError::Conflict(
                    msg.clone().unwrap_or_else(|| "constraint violation".into()),
                );
            }
        }
        StoreError::Db(err.into())
    }
}

impl From<r2d2::Error> for StoreError {
    fn from(err: r2d2::Error) -> Self {
        StoreError::Db(err.into())
    }
}

#[cfg(feature = "cloud")]
impl From<postgres::Error> for StoreError {
    fn from(err: postgres::Error) -> Self {
        // A unique/foreign-key violation surfaces as a conflict, mirroring the
        // SQLite constraint mapping above; everything else is an internal error.
        if let Some(db) = err.as_db_error() {
            if db.code() == &postgres::error::SqlState::UNIQUE_VIOLATION
                || db.code() == &postgres::error::SqlState::FOREIGN_KEY_VIOLATION
            {
                return StoreError::Conflict(db.message().to_string());
            }
        }
        StoreError::Db(err.into())
    }
}
