//! POST /api/v1/orgs/{org}/rules — create a stored rule in an org, optionally
//! pinned to a site (`site_id` in the body).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::validate_slug;
use uuid::Uuid;

use super::dto::{CreateRule, RuleView};
use crate::api::blocking::blocking;
use crate::api::scope_auth::authorize_rule_write;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::store::RuleRecord;
use crate::AppState;

#[utoipa::path(post, path = "/api/v1/orgs/{org}/rules", request_body = CreateRule, tag = "rules",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 201, body = RuleView), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_rule(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
    Json(req): Json<CreateRule>,
) -> Result<(StatusCode, Json<RuleView>), ApiError> {
    validate_slug(&org)?;
    validate_slug(&req.name)?;
    authorize_rule_write(&principal, &state.store, &org, req.site_id, "*")?;
    let rule = RuleRecord {
        id: Uuid::new_v4(),
        org,
        site_id: req.site_id,
        name: req.name,
        script: req.script,
        params: req.params,
        created_at: Utc::now(),
    };
    let stored = rule.clone();
    blocking(move || Ok(state.store.create_rule(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(rule.into())))
}
