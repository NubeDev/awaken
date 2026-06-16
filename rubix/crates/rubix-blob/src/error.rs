//! The blob store's error type.

/// Why a blob operation failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BlobError {
    /// The requested blob does not exist (unknown id, or not in this namespace).
    #[error("blob not found")]
    NotFound,

    /// A backend was requested that this build does not carry (fail closed).
    ///
    /// The object-store backend lives behind the `cloud` feature; requesting it
    /// without that feature is a deliberate, fail-closed refusal — never a silent
    /// fallback to a different store.
    #[error("blob backend unavailable: {0}")]
    BackendUnavailable(String),

    /// The blob id was malformed (empty, or contains a path separator).
    ///
    /// Ids are minted server-side and used as path segments, so a separator or an
    /// empty id is rejected rather than allowed to escape the store root.
    #[error("invalid blob id: {0}")]
    InvalidId(String),

    /// The namespace was malformed (empty, or contains a path separator).
    #[error("invalid namespace: {0}")]
    InvalidNamespace(String),

    /// The stored blob's sidecar metadata could not be read back.
    #[error("blob metadata is corrupt: {0}")]
    CorruptMetadata(String),

    /// An underlying I/O operation failed.
    #[error("blob i/o failed: {0}")]
    Io(String),
}
