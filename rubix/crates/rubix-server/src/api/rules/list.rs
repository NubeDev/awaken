//! GET /api/v1/orgs/{org}/rules — list an org's stored rules.

use axum::extract::{Path, State};
use axum::Json;

use super::dto::RuleView;
use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::ApiError;
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/orgs/{org}/rules", tag = "rules",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 200, body = [RuleView])))]
pub(crate) async fn list_rules(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<RuleView>>, ApiError> {
    principal.authorize_read(&Scope::org(&org))?;
    let rules = blocking(move || Ok(state.store.list_rules(&org)?)).await?;
    Ok(Json(rules.into_iter().map(RuleView::from).collect()))
}
