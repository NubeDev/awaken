//! `GET /records` — list the records visible to the principal's session.
//!
//! A read runs on the WS-03 scoped session: SurrealDB row-level permissions
//! return only the principal's namespace records, with no app filter (contract
//! #1). This is also the surface a dashboard reads recorded insights through —
//! insights are generic records (`rubix-rules` records them as such), so they
//! appear here scoped to the principal.

use axum::Json;
use rubix_gate::read_records_on_session;

use crate::auth::Authenticated;
use crate::dto::record::RecordDto;
use crate::error::{ApiError, ApiResult};

/// List every record the principal's session may read, as DTOs.
pub async fn list_records_route(auth: Authenticated) -> ApiResult<Json<Vec<RecordDto>>> {
    let records = read_records_on_session(&auth.session)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(records.into_iter().map(RecordDto::from).collect()))
}
