//! `GET /files/:id` — stream a stored blob's bytes back to the caller.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::HeaderValue;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::Response;

use crate::auth::Authenticated;
use crate::error::ApiResult;
use crate::http::files::map_blob_error;
use crate::state::AppState;

/// Stream the blob `id` owned by the caller's namespace.
///
/// The blob is loaded under the principal's own namespace, so a caller can only
/// retrieve files belonging to its tenant — a guessed id in another namespace
/// resolves to `404`. The stored content type and filename are echoed back so a
/// browser renders or downloads the file correctly.
pub async fn download_file_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let loaded = state
        .blobs
        .load(&auth.principal.namespace, &id)
        .await
        .map_err(map_blob_error)?;

    let mut response = Response::new(Body::from(loaded.bytes));
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&loaded.reference.content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    // `attachment; filename="…"` with the stored name; fall back to a safe default
    // if the filename cannot form a valid header value.
    if let Ok(disposition) = HeaderValue::from_str(&format!(
        "attachment; filename=\"{}\"",
        loaded.reference.filename
    )) {
        headers.insert(CONTENT_DISPOSITION, disposition);
    }
    Ok(response)
}
