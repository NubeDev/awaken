//! `GET /records/:id` — read a record on the principal's scoped session.
//!
//! A read runs on the WS-03 gate-issued scoped session, so SurrealDB row-level
//! permissions decide visibility — a record outside the principal's namespace
//! resolves to `None` natively, not by an app filter (contract #1,
//! `rubix/STACK-DEISGN.md`). The handler extracts the id, reads on the session,
//! and maps the record to its DTO.

use axum::Json;
use axum::extract::Path;
use rubix_core::Id;
use rubix_gate::read_record_on_session;

use crate::auth::Authenticated;
use crate::dto::record::RecordDto;
use crate::error::{ApiError, ApiResult};

/// Read the record `id` if the principal's session may see it, else `404`.
pub async fn get_record_route(
    auth: Authenticated,
    Path(id): Path<String>,
) -> ApiResult<Json<RecordDto>> {
    let record = read_record_on_session(&auth.session, &Id::from_raw(id))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(record.into()))
}
