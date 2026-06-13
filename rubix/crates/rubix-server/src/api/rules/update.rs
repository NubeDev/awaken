//! PUT /api/v1/orgs/{org}/rules/{name} — replace a rule's script and params.

use axum::extract::{Path, State};
use axum::Json;

use super::dto::{RuleView, UpdateRule};
use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(put, path = "/api/v1/orgs/{org}/rules/{name}", request_body = UpdateRule, tag = "rules",
    params(("org" = String, Path, description = "Tenant org"),
           ("name" = String, Path, description = "Rule name")),
    security(("bearer" = [])),
    responses((status = 200, body = RuleView), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn update_rule(
    State(state): State<AppState>,
    Path((org, name)): Path<(String, String)>,
    principal: RequestPrincipal,
    Json(req): Json<UpdateRule>,
) -> Result<Json<RuleView>, ApiError> {
    principal.authorize_write(&Scope::org(&org))?;
    let rule = blocking(move || {
        Ok(state
            .store
            .update_rule(&org, &name, &req.script, &req.params)?)
    })
    .await?;
    Ok(Json(rule.into()))
}
