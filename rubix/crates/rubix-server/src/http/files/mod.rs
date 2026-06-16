//! File-blob routes — upload bytes, download bytes.
//!
//! The two-step file contract (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "File
//! fields"): a client uploads bytes to the blob store with `POST /files` (gated on
//! `FileUpload`) and gets back a [`FileRef`](rubix_blob::FileRef); it then stores
//! that reference in a record's `file` field through the normal gated
//! `POST /records`. So blob bytes never cross the command gate — only the JSON
//! reference does. Downloads stream the bytes back, scoped to the caller's
//! namespace so one tenant cannot read another's files.

mod download;
mod upload;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

pub use download::download_file_route;
pub use upload::upload_file_route;

/// Map an [`rubix_blob::BlobError`] to its transport status.
///
/// A missing blob is `404`; a malformed id/namespace is `400` (it never reaches a
/// real blob); an unavailable backend (object store without its feature) is `500`
/// — a deployment fault, fail closed, not the caller's error.
pub(crate) fn map_blob_error(error: rubix_blob::BlobError) -> crate::error::ApiError {
    use crate::error::ApiError;
    use rubix_blob::BlobError;
    match error {
        BlobError::NotFound => ApiError::NotFound,
        BlobError::InvalidId(reason) | BlobError::InvalidNamespace(reason) => {
            ApiError::BadRequest(reason)
        }
        other => ApiError::Internal(other.to_string()),
    }
}

/// The file routes: `POST /files` (multipart upload) and `GET /files/:id`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/files", post(upload_file_route))
        .route("/files/:id", get(download_file_route))
}
