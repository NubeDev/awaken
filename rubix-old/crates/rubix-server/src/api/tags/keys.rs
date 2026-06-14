//! GET /api/v1/tags/keys?org=…&kind=… — distinct tag keys in use, for authoring
//! autocomplete. Org-scoped read.

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use super::{parse_kind, require_read_scope_org};
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct KeysQuery {
    /// The tenant org the lookup is scoped to.
    pub org: String,
    /// The entity kind whose keys to list (e.g. `dashboard`).
    pub kind: String,
}

#[utoipa::path(get, path = "/api/v1/tags/keys", tag = "tags", params(KeysQuery),
    security(("bearer" = [])),
    responses((status = 200, body = [String]), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn tag_keys(
    State(state): State<AppState>,
    Query(q): Query<KeysQuery>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<String>>, ApiError> {
    let kind = parse_kind(&q.kind)?;
    require_read_scope_org(&principal, &q.org)?;
    let org = q.org;
    let keys = blocking(move || Ok(state.store.entity_tag_keys(&org, kind.as_str())?)).await?;
    Ok(Json(keys))
}
