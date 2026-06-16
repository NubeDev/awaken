//! The file-reference wire shape returned by an upload.
//!
//! A `POST /files` returns the reference a client then stores in a record's `file`
//! field through the normal gated write (`rubix/docs/design/BACKEND-COLLECTIONS.md`,
//! "File fields"). The domain type is [`rubix_blob::FileRef`]; this DTO is its
//! OpenAPI-documented projection so a typed client can generate against it.

use rubix_blob::FileRef;
use serde::Serialize;
use utoipa::ToSchema;

/// A reference to an uploaded blob — the body of an upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileRefDto {
    /// The server-minted blob id — the `GET /files/:id` handle.
    pub id: String,
    /// The original filename (display only).
    pub filename: String,
    /// The blob size in bytes.
    pub size: u64,
    /// The blob's MIME content type.
    pub content_type: String,
}

impl From<FileRef> for FileRefDto {
    fn from(reference: FileRef) -> Self {
        Self {
            id: reference.id,
            filename: reference.filename,
            size: reference.size,
            content_type: reference.content_type,
        }
    }
}
