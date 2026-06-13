//! PUT /api/v1/orgs/{org}/rules/{name}?site_id= — replace a rule's script and
//! params at an exact scope.

use axum::extract::{Path, Query, State};
use axum::Json;

use super::dto::{RuleScope, RuleView, UpdateRule};
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_scope_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(put, path = "/api/v1/orgs/{org}/rules/{name}", request_body = UpdateRule, tag = "rules",
    params(("org" = String, Path, description = "Tenant org"),
           ("name" = String, Path, description = "Rule name"), RuleScope),
    security(("bearer" = [])),
    responses((status = 200, body = RuleView), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn update_rule(
    State(state): State<AppState>,
    Path((org, name)): Path<(String, String)>,
    Query(scope): Query<RuleScope>,
    principal: RequestPrincipal,
    Json(req): Json<UpdateRule>,
) -> Result<Json<RuleView>, ApiError> {
    authorize_scope_write(&principal, &state.store, &org, scope.site_id)?;
    let rule = blocking(move || {
        Ok(state
            .store
            .update_rule(&org, scope.site_id, &name, &req.script, &req.params)?)
    })
    .await?;
    Ok(Json(rule.into()))
}
