//! GET /api/v1/audit/{kind}/{id} — the change timeline for one resource
//! (docs/design/audit-and-undo.md "Audit read surface"), powering a per-resource
//! "History" tab. Org-scoped and admin-gated like the list query; `org` is a
//! required query param so the gate has a tenant to authorize against.

use axum::extract::{Path, Query, State};
use axum::Json;
use rubix_core::Change;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

/// The tenant whose resource timeline is read; the caller must be its admin.
#[derive(Debug, Deserialize, IntoParams)]
pub struct TimelineQuery {
    pub org: String,
}

#[utoipa::path(get, path = "/api/v1/audit/{kind}/{id}",
    params(("kind" = String, Path), ("id" = Uuid, Path), TimelineQuery),
    tag = "audit", security(("bearer" = [])),
    responses((status = 200, body = [Change]), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody)))]
pub(crate) async fn resource_timeline(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path((kind, id)): Path<(String, Uuid)>,
    Query(q): Query<TimelineQuery>,
) -> Result<Json<Vec<Change>>, ApiError> {
    principal.require_admin(&q.org)?;
    let store = state.store.clone();
    let rows = blocking(move || Ok(store.resource_changes(&q.org, &kind, id)?)).await?;
    Ok(Json(rows))
}
