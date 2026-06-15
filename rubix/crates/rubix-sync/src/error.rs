//! The sync-crate error domain.
//!
//! `rubix-sync` owns the edge↔cloud shipper (`rubix/STACK-DEISGN.md`,
//! `rubix-sync` row; `rubix/docs/SCOPE.md`, "Sync and conflict model"). SurrealDB
//! has no mature multi-master replication, so sync is an application-level shipper
//! over Zenoh, not DB replication. Its failures are distinct from the store/gate
//! domains it composes: a Zenoh session/publish failure on the wire, a record that
//! could not be encoded/decoded for the wire, a receiver-side write that failed to
//! land, and a config reconcile that could not decide an owner. Each converts into
//! the project [`Error`](rubix_core::Error) at the crate boundary so callers chain
//! with `.context()` (CLAUDE.md "Key Patterns").

use rubix_core::Error;

/// Convenience alias for the sync-crate result.
pub type Result<T> = std::result::Result<T, SyncError>;

/// A failure raised while shipping data records or reconciling config.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SyncError {
    /// Opening the Zenoh session or declaring the publisher failed.
    #[error("zenoh session error: {0}")]
    Session(String),

    /// Publishing a record onto the sync key-space failed.
    #[error("publish error: {0}")]
    Publish(String),

    /// A record could not be encoded for the wire or decoded from it.
    #[error("wire codec error: {0}")]
    Codec(String),

    /// Landing a shipped record into the receiver namespace failed.
    #[error("apply error: {0}")]
    Apply(String),
}

impl From<SyncError> for Error {
    fn from(error: SyncError) -> Self {
        Error::Store(error.to_string())
    }
}
