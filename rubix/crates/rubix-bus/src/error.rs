//! Bus-domain errors, converted into the project error at the crate boundary.
//!
//! The bus has two failure surfaces: opening a live-query subscription on a
//! scoped session (a SurrealDB call) and decoding a data-change notification.
//! In-process control fan-out cannot fail on publish — a broadcast to zero
//! subscribers is a no-op, not an error (`rubix/docs/SCOPE.md`, "Event bus").

use rubix_core::Error as CoreError;

/// Convenience alias for bus results.
pub type Result<T> = std::result::Result<T, BusError>;

/// Failures raised by the event bus.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BusError {
    /// Opening a live-query subscription against the scoped session failed.
    #[error("failed to open live-query subscription: {0}")]
    Subscribe(#[source] surrealdb::Error),

    /// A live-query notification carried a record the bus could not decode into
    /// a domain [`Record`](rubix_core::Record).
    #[error("failed to decode data-change notification: {0}")]
    Decode(String),

    /// The data plane reported a live-query evaluation error rather than a
    /// record change (SurrealDB `Action::Error`); the message is surfaced so a
    /// subscriber can diagnose a broken query.
    #[error("live-query evaluation error: {0}")]
    Evaluation(String),
}

impl From<BusError> for CoreError {
    fn from(err: BusError) -> Self {
        CoreError::Store(err.to_string())
    }
}
