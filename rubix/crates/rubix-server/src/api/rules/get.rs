//! GET /api/v1/orgs/{org}/rules/{name}?site_id= — load one stored rule at an
//! exact scope (no org fallback, so an admin sees the rule they addressed).

use axum::extract::{Path, Query, State};
use axum::Json;

use super::dto::{RuleScope, RuleView};
use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/orgs/{org}/rules/{name}", tag = "rules",
    params(("org" = String, Path, description = "Tenant org"),
           ("name" = String, Path, description = "Rule name"), RuleScope),
    security(("bearer" = [])),
    responses((status = 200, body = RuleView), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_rule(
    State(state): State<AppState>,
    Path((org, name)): Path<(String, String)>,
    Query(scope): Query<RuleScope>,
    principal: RequestPrincipal,
) -> Result<Json<RuleView>, ApiError> {
    principal.authorize_read(&Scope::org(&org))?;
    let rule =
        blocking(move || Ok(state.store.load_rule_exact(&org, scope.site_id, &name)?)).await?;
    Ok(Json(rule.into()))
}
