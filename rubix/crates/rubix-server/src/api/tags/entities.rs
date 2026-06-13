//! GET /api/v1/tags/entities/{kind}?org=… — reverse lookup: which entities of a
//! kind carry tags, with their full sets. Org-scoped (the caller must read the
//! org). Drives "which boards hold the `building` tag" in authoring.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::Json;
use rubix_core::EntityTags;
use serde::Deserialize;
use utoipa::IntoParams;

use super::{parse_kind, require_read_scope_org};
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[derive(Debug, Deserialize, IntoParams)]
pub(crate) struct OrgQuery {
    /// The tenant org the lookup is scoped to.
    pub org: String,
}

#[utoipa::path(get, path = "/api/v1/tags/entities/{kind}", tag = "tags",
    params(("kind" = String, Path, description = "Entity kind (e.g. dashboard)"), OrgQuery),
    security(("bearer" = [])),
    responses((status = 200, body = Object), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn tagged_entities(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Query(q): Query<OrgQuery>,
    principal: RequestPrincipal,
) -> Result<Json<BTreeMap<String, EntityTags>>, ApiError> {
    let kind = parse_kind(&kind)?;
    require_read_scope_org(&principal, &q.org)?;
    let org = q.org;
    let rows = blocking(move || Ok(state.store.entities_with_tags(&org, kind.as_str())?)).await?;
    let map = rows
        .into_iter()
        .map(|(id, tags)| (id.to_string(), tags))
        .collect();
    Ok(Json(map))
}
