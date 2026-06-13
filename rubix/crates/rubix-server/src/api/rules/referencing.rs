//! GET /api/v1/orgs/{org}/rules/{name}/referencing — the change-impact listing.
//!
//! Composition resolves live by name in v1, so editing a shared rule changes
//! every rule built on it on the next tick. This lists the rules in the org that
//! compose `{name}` so an operator can see the blast radius before editing it.

use axum::extract::{Path, State};
use axum::Json;

use super::dto::RuleView;
use crate::api::blocking::blocking;
use crate::auth::{RequestPrincipal, Scope};
use crate::error::ApiError;
use crate::AppState;

#[utoipa::path(get, path = "/api/v1/orgs/{org}/rules/{name}/referencing", tag = "rules",
    params(("org" = String, Path, description = "Tenant org"),
           ("name" = String, Path, description = "Rule name composed by the results")),
    security(("bearer" = [])),
    responses((status = 200, body = [RuleView])))]
pub(crate) async fn referencing_rules(
    State(state): State<AppState>,
    Path((org, name)): Path<(String, String)>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<RuleView>>, ApiError> {
    principal.authorize_read(&Scope::org(&org))?;
    let rules = blocking(move || Ok(state.store.referencing_rules(&org, &name)?)).await?;
    Ok(Json(rules.into_iter().map(RuleView::from).collect()))
}
