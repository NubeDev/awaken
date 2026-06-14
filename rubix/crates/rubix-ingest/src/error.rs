//! The ingest-crate error domain.
//!
//! `rubix-ingest` owns the streaming ingestion path (`rubix/STACK-DEISGN.md`,
//! `rubix-ingest` row; `rubix/docs/SCOPE.md`, "Ingestion and pre-processing"):
//! a Zenoh subscriber scoped once at subscribe by the gate, in-flight
//! decimate/filter/enrich nodes, and append-only edge-partitioned persistence.
//! Its failures are distinct from the gate/store domains it composes: the gate
//! refusing the key-space subscribe (fail closed, contract #2), a malformed
//! key-space scope, a Zenoh session/subscribe failure, a sample whose payload is
//! not the expected record content, and the wrapped failure of the persistence
//! write. Each converts into the project [`Error`](rubix_core::Error) at the
//! crate boundary so callers chain with `.context()` (CLAUDE.md "Key Patterns").

use rubix_core::Error;

/// Convenience alias for the ingest-crate result.
pub type Result<T> = std::result::Result<T, IngestError>;

/// A failure raised while authorizing, subscribing, pre-processing, or
/// persisting an ingested stream.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IngestError {
    /// The gate refused the key-space subscribe — the principal lacks the
    /// `zenoh-subscribe` grant, or the requested scope escapes its partition.
    #[error("subscribe denied: {0}")]
    Denied(String),

    /// The requested key-space is not a valid Zenoh key expression.
    #[error("invalid key-space: {0}")]
    KeySpace(String),

    /// Opening the Zenoh session or declaring the subscriber failed.
    #[error("zenoh session error: {0}")]
    Session(String),

    /// A received sample could not be decoded into record content.
    #[error("sample decode error: {0}")]
    Sample(String),

    /// Persisting an ingested record through the store failed.
    #[error("persist error: {0}")]
    Persist(String),
}

impl From<IngestError> for Error {
    fn from(error: IngestError) -> Self {
        Error::Store(error.to_string())
    }
}
