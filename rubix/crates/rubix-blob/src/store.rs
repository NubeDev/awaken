//! The pluggable blob store boundary.

use async_trait::async_trait;

use crate::error::BlobError;
use crate::reference::FileRef;

/// A blob loaded from the store: its reference metadata plus the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The stored reference (filename, size, content type) for response headers.
    pub reference: FileRef,
    /// The blob's bytes.
    pub bytes: Vec<u8>,
}

/// A namespace-scoped store for binary blobs.
///
/// Every operation is keyed by `(namespace, blob_id)`: a blob belongs to exactly
/// one namespace and is only reachable through it, so the download route enforces
/// tenant isolation by passing the caller's own namespace. Implementations must be
/// `Send + Sync` so the store lives in shared server state behind an `Arc`.
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Store `bytes` under `reference.id` in `namespace`, recording `reference`'s
    /// metadata alongside so a later [`load`](BlobStore::load) can rebuild it.
    ///
    /// # Errors
    /// [`BlobError::InvalidNamespace`]/[`BlobError::InvalidId`] if either key is
    /// malformed, or [`BlobError::Io`] if the write fails.
    async fn put(&self, namespace: &str, reference: &FileRef, bytes: &[u8])
    -> Result<(), BlobError>;

    /// Load the blob `blob_id` in `namespace` — its reference and bytes.
    ///
    /// # Errors
    /// [`BlobError::NotFound`] if no such blob exists in the namespace,
    /// [`BlobError::CorruptMetadata`] if the sidecar cannot be read, or
    /// [`BlobError::Io`] on a read failure.
    async fn load(&self, namespace: &str, blob_id: &str) -> Result<Loaded, BlobError>;

    /// Delete the blob `blob_id` in `namespace`. Deleting a missing blob is a
    /// no-op (idempotent), so an orphan sweep can call it freely.
    ///
    /// # Errors
    /// [`BlobError::Io`] if the delete fails for a reason other than absence.
    async fn delete(&self, namespace: &str, blob_id: &str) -> Result<(), BlobError>;
}

/// Reject a namespace or id that is empty or could escape the store root.
///
/// Ids and namespaces become path segments in the filesystem backend, so a
/// separator (`/`, `\`) or `..` could traverse out of the root; they are rejected
/// up front so no backend has to re-check. Ids are minted server-side, so a
/// rejection here means a programming error, not user input.
pub(crate) fn validate_key(value: &str, kind: KeyKind) -> Result<(), BlobError> {
    let bad = value.is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value == ".."
        || value == ".";
    if bad {
        return Err(match kind {
            KeyKind::Namespace => BlobError::InvalidNamespace(value.to_owned()),
            KeyKind::Id => BlobError::InvalidId(value.to_owned()),
        });
    }
    Ok(())
}

/// Which key is being validated, for the right error variant.
pub(crate) enum KeyKind {
    Namespace,
    Id,
}
