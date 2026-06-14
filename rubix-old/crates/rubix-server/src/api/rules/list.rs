//! GET /api/v1/orgs/{org}/rules?site_id= — list an org's stored rules. With
//! `?site_id=`, returns that site's rules plus the org-level ones (the set that
//! resolves on the site); without, every rule the org owns.

use axum::extract::{Path, Query, State};
use axum::Json;

use super::dto::{RuleScope, RuleView};
use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::ApiError;
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/orgs/{org}/rules", tag = "rules",
    params(("org" = String, Path, description = "Tenant org"), RuleScope),
    security(("bearer" = [])),
    responses((status = 200, body = [RuleView])))]
pub(crate) async fn list_rules(
    State(state): State<AppState>,
    Path(org): Path<String>,
    Query(scope): Query<RuleScope>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<RuleView>>, ApiError> {
    principal.authorize_read(&Scope::org(&org))?;
    let rules = blocking(move || Ok(state.store.list_rules(&org, scope.site_id)?)).await?;
    Ok(Json(rules.into_iter().map(RuleView::from).collect()))
}
