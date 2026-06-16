//! The local-filesystem blob store — the edge default.

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::BlobError;
use crate::reference::FileRef;
use crate::store::{BlobStore, KeyKind, Loaded, validate_key};

/// A blob store rooted at a local directory.
///
/// Layout is `{root}/{namespace}/{id}` for the bytes and `{root}/{namespace}/
/// {id}.meta.json` for the reference sidecar (filename, size, content type). The
/// per-namespace directory both scopes ownership and keeps one tenant's blobs out
/// of another's listing. This is the default backend, matching the embedded-engine
/// edge profile (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "File fields").
#[derive(Debug, Clone)]
pub struct LocalFsBlobStore {
    root: PathBuf,
}

impl LocalFsBlobStore {
    /// Root a store at `root`, creating the directory tree on first write.
    #[must_use]
    pub fn open(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory holding `namespace`'s blobs.
    fn namespace_dir(&self, namespace: &str) -> PathBuf {
        self.root.join(namespace)
    }

    /// The bytes path for a blob.
    fn blob_path(&self, namespace: &str, blob_id: &str) -> PathBuf {
        self.namespace_dir(namespace).join(blob_id)
    }

    /// The sidecar metadata path for a blob.
    fn meta_path(&self, namespace: &str, blob_id: &str) -> PathBuf {
        self.namespace_dir(namespace)
            .join(format!("{blob_id}.meta.json"))
    }
}

#[async_trait]
impl BlobStore for LocalFsBlobStore {
    async fn put(
        &self,
        namespace: &str,
        reference: &FileRef,
        bytes: &[u8],
    ) -> Result<(), BlobError> {
        validate_key(namespace, KeyKind::Namespace)?;
        validate_key(&reference.id, KeyKind::Id)?;

        let dir = self.namespace_dir(namespace);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(io_to_blob)?;

        // Write the sidecar first, then the bytes: a reader treats a missing
        // sidecar as "not found", so bytes are never visible without their
        // metadata.
        let meta = serde_json::to_vec(reference).map_err(|e| BlobError::Io(e.to_string()))?;
        write_atomic(&self.meta_path(namespace, &reference.id), &meta).await?;
        write_atomic(&self.blob_path(namespace, &reference.id), bytes).await?;
        Ok(())
    }

    async fn load(&self, namespace: &str, blob_id: &str) -> Result<Loaded, BlobError> {
        validate_key(namespace, KeyKind::Namespace)?;
        validate_key(blob_id, KeyKind::Id)?;

        let meta_bytes = match tokio::fs::read(self.meta_path(namespace, blob_id)).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BlobError::NotFound);
            }
            Err(error) => return Err(io_to_blob(error)),
        };
        let reference: FileRef = serde_json::from_slice(&meta_bytes)
            .map_err(|e| BlobError::CorruptMetadata(e.to_string()))?;

        let bytes = match tokio::fs::read(self.blob_path(namespace, blob_id)).await {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(BlobError::NotFound);
            }
            Err(error) => return Err(io_to_blob(error)),
        };
        Ok(Loaded { reference, bytes })
    }

    async fn delete(&self, namespace: &str, blob_id: &str) -> Result<(), BlobError> {
        validate_key(namespace, KeyKind::Namespace)?;
        validate_key(blob_id, KeyKind::Id)?;
        remove_if_present(&self.blob_path(namespace, blob_id)).await?;
        remove_if_present(&self.meta_path(namespace, blob_id)).await?;
        Ok(())
    }
}

/// Write `bytes` to `path` via a temp file + rename, so a reader never sees a
/// half-written blob (the rename is atomic on a single filesystem).
async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), BlobError> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await.map_err(io_to_blob)?;
    tokio::fs::rename(&tmp, path).await.map_err(io_to_blob)?;
    Ok(())
}

/// Remove `path`, treating absence as success (idempotent delete).
async fn remove_if_present(path: &Path) -> Result<(), BlobError> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_to_blob(error)),
    }
}

/// Map an I/O error to [`BlobError::Io`].
fn io_to_blob(error: std::io::Error) -> BlobError {
    BlobError::Io(error.to_string())
}
