//! Bridge from async handlers to the synchronous SQLite store.

use crate::error::ApiError;

pub(crate) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
}
