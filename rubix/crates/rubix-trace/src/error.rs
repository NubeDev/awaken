//! Trace-domain errors, converted into the project error at the crate boundary.
//!
//! Tracing has two durable failure surfaces: appending a span to the bounded
//! `trace` table and reading spans back to assemble a tree. Sampling and bus
//! emission cannot fail — a dropped span is a deliberate no-op and a control-bus
//! publish to zero subscribers is a no-op too (`rubix/docs/SCOPE.md`, "Event
//! bus"; contract #4 in `rubix/STACK-DEISGN.md`).

use rubix_core::Error as CoreError;

/// Convenience alias for trace results.
pub type Result<T> = std::result::Result<T, TraceError>;

/// Failures raised by the tracing crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TraceError {
    /// Appending a span to the `trace` table failed.
    #[error("failed to persist span: {0}")]
    Persist(#[source] surrealdb::Error),

    /// Enforcing the rolling retention bound on the `trace` table failed.
    #[error("failed to enforce trace retention: {0}")]
    Retain(#[source] surrealdb::Error),

    /// Reading spans back to assemble a tree failed.
    #[error("failed to read spans for trace: {0}")]
    Assemble(#[source] surrealdb::Error),

    /// Defining the `trace` table schema failed.
    #[error("failed to define trace schema: {0}")]
    DefineSchema(#[source] surrealdb::Error),
}

impl From<TraceError> for CoreError {
    fn from(err: TraceError) -> Self {
        CoreError::Store(err.to_string())
    }
}
