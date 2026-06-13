//! DELETE /api/v1/orgs/{org}/rules/{name} — remove a stored rule.

use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

#[utoipa::path(delete, path = "/api/v1/orgs/{org}/rules/{name}", tag = "rules",
    params(("org" = String, Path, description = "Tenant org"),
           ("name" = String, Path, description = "Rule name")),
    security(("bearer" = [])),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_rule(
    State(state): State<AppState>,
    Path((org, name)): Path<(String, String)>,
    principal: RequestPrincipal,
) -> Result<StatusCode, ApiError> {
    principal.authorize_write(&Scope::org(&org))?;
    blocking(move || Ok(state.store.delete_rule(&org, &name)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}
