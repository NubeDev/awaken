//! `POST /files` — upload a file's bytes into the blob store.

use axum::Json;
use axum::extract::{Multipart, State};
use rubix_blob::FileRef;
use rubix_gate::{Capability, check_capability};

use crate::auth::Authenticated;
use crate::dto::FileRefDto;
use crate::error::{ApiError, ApiResult};
use crate::http::files::map_blob_error;
use crate::state::AppState;

/// Store the first file part of a multipart body and return its reference.
///
/// A caller lacking the `file-upload` grant gets `403` before any bytes are read
/// (fail closed). The blob id is minted server-side (never from the client), and
/// the bytes are stored under the caller's namespace, so the returned reference is
/// only retrievable by that tenant. The client then writes the reference into a
/// record's `file` field through the normal gated `POST /records` — this endpoint
/// never touches the command gate.
pub async fn upload_file_route(
    State(state): State<AppState>,
    auth: Authenticated,
    mut multipart: Multipart,
) -> ApiResult<Json<FileRefDto>> {
    if !check_capability(state.store.raw(), &auth.principal, Capability::FileUpload)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        return Err(ApiError::Forbidden(
            "upload requires the file-upload capability".to_owned(),
        ));
    }

    let field = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("malformed multipart body: {e}")))?
        .ok_or_else(|| ApiError::BadRequest("multipart body carried no file part".to_owned()))?;

    // Capture filename/content-type before consuming the field into bytes (the
    // borrow ends once `bytes()` is called).
    let filename = field
        .file_name()
        .map(str::to_owned)
        .unwrap_or_else(|| "upload".to_owned());
    let content_type = field
        .content_type()
        .map(str::to_owned)
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    let bytes = field
        .bytes()
        .await
        .map_err(|e| ApiError::BadRequest(format!("could not read file bytes: {e}")))?;

    let id = uuid::Uuid::new_v4().to_string();
    let reference = FileRef::new(id, filename, bytes.len() as u64, content_type);
    state
        .blobs
        .put(&auth.principal.namespace, &reference, &bytes)
        .await
        .map_err(map_blob_error)?;

    Ok(Json(reference.into()))
}
