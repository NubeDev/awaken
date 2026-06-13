//! GET /api/v1/orgs/{org}/rules/{name} — load one stored rule by name.

use axum::extract::{Path, State};
use axum::Json;

use super::dto::RuleView;
use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/orgs/{org}/rules/{name}", tag = "rules",
    params(("org" = String, Path, description = "Tenant org"),
           ("name" = String, Path, description = "Rule name")),
    security(("bearer" = [])),
    responses((status = 200, body = RuleView), (status = 404, body = ErrorBody)))]
pub(crate) async fn get_rule(
    State(state): State<AppState>,
    Path((org, name)): Path<(String, String)>,
    principal: RequestPrincipal,
) -> Result<Json<RuleView>, ApiError> {
    principal.authorize_read(&Scope::org(&org))?;
    let rule = blocking(move || Ok(state.store.load_rule(&org, &name)?)).await?;
    Ok(Json(rule.into()))
}
