//! Store-domain errors, converted into the project error at the crate boundary.

use rubix_core::Error as CoreError;

/// Convenience alias for store results.
pub type Result<T> = std::result::Result<T, StoreError>;

/// Failures raised by the SurrealDB store boundary.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// Opening the embedded engine failed.
    #[error("failed to open store engine: {0}")]
    Connect(#[source] surrealdb::Error),

    /// Selecting or creating the namespace/database failed.
    #[error("failed to bootstrap namespace/database: {0}")]
    Bootstrap(#[source] surrealdb::Error),

    /// A health probe against the engine failed.
    #[error("store health probe failed: {0}")]
    Health(#[source] surrealdb::Error),

    /// A read or write through the durable boundary failed.
    #[error("store operation failed: {0}")]
    Operation(#[source] surrealdb::Error),
}

impl From<StoreError> for CoreError {
    fn from(err: StoreError) -> Self {
        CoreError::Store(err.to_string())
    }
}
