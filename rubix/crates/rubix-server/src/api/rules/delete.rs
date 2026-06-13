//! DELETE /api/v1/orgs/{org}/rules/{name}?site_id= — remove a stored rule at an
//! exact scope.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use super::dto::RuleScope;
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_rule_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/orgs/{org}/rules/{name}", tag = "rules",
    params(("org" = String, Path, description = "Tenant org"),
           ("name" = String, Path, description = "Rule name"), RuleScope),
    security(("bearer" = [])),
    responses((status = 204), (status = 401, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_rule(
    State(state): State<AppState>,
    Path((org, name)): Path<(String, String)>,
    Query(scope): Query<RuleScope>,
    principal: RequestPrincipal,
) -> Result<StatusCode, ApiError> {
    authorize_rule_write(&principal, &state.store, &org, scope.site_id, &name)?;
    blocking(move || Ok(state.store.delete_rule(&org, scope.site_id, &name)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
