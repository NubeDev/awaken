//! GET /api/v1/audit — the org-scoped, capability-gated audit query
//! (docs/design/audit-and-undo.md "Audit read surface"). Returns `Change` rows
//! newest-first with `before`/`after` for diff rendering, narrowed by the optional
//! filters. Privileged: an admin of the queried `org` only — a cross-org read is
//! impossible because the store query always filters by `org` and the gate demands
//! an admin covering it.

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::{Change, Op};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::store::ChangeFilter;
use crate::AppState;

/// Audit query filters. `org` is required and always enforced; the rest narrow the
/// result. `limit` caps the page (defaulted by the store when 0/absent).
#[derive(Debug, Deserialize, IntoParams)]
pub struct AuditQuery {
    /// Tenant whose log to read. The caller must be an admin covering it.
    pub org: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub resource_id: Option<Uuid>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub op: Option<Op>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[utoipa::path(get, path = "/api/v1/audit", params(AuditQuery), tag = "audit",
    security(("bearer" = [])),
    responses((status = 200, body = [Change]), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody)))]
pub(crate) async fn list_audit(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<Change>>, ApiError> {
    // Audit read is privileged: an admin of the queried org (or a super-admin).
    principal.require_admin(&q.org)?;
    let filter = ChangeFilter {
        kind: q.kind,
        resource_id: q.resource_id,
        actor_subject: q.actor,
        op: q.op,
        limit: q.limit.unwrap_or(0),
    };
    let store = state.store.clone();
    let rows = blocking(move || Ok(store.list_changes(&q.org, &filter)?)).await?;
    Ok(Json(rows))
}
