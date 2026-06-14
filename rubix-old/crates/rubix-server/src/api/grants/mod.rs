//! Per-resource grant management (Layer-2 ACL):
//! `/api/v1/orgs/{org}/grants` and the convenience `/api/v1/dashboards/{id}/grants`.
//! Admin-gated. A grant ADDS access for a user or team on a resource (or `*`).
//! See `docs/design/authz-rbac.md` increment C.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::api::scope_auth::resource_ref;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::store::{GrantRecord, Permission, SubjectKind};
use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/orgs/{org}/grants",
            get(list_grants).post(create_grant),
        )
        .route(
            "/api/v1/orgs/{org}/grants/{id}",
            axum::routing::delete(delete_grant),
        )
        .route(
            "/api/v1/dashboards/{id}/grants",
            get(list_dashboard_grants).post(create_dashboard_grant),
        )
}

/// Create-grant body. `resource_kind` + `resource_ref` address the target
/// (`resource_ref = "*"` for all-of-kind within the org).
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateGrant {
    #[schema(value_type = String)]
    pub subject_kind: SubjectKind,
    /// The user or team id (a UUID string).
    pub subject_id: String,
    pub resource_kind: String,
    pub resource_ref: String,
    #[schema(value_type = String)]
    pub permission: Permission,
}

/// Grant body addressed at the dashboard in the path (kind/ref are implied).
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateDashboardGrant {
    #[schema(value_type = String)]
    pub subject_kind: SubjectKind,
    pub subject_id: String,
    #[schema(value_type = String)]
    pub permission: Permission,
}

#[derive(Debug, Default, Deserialize, IntoParams)]
pub(crate) struct GrantFilter {
    /// Filter to grants on this exact `resource_ref`.
    #[serde(default)]
    pub resource_ref: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/orgs/{org}/grants", tag = "grants",
    params(("org" = String, Path, description = "Tenant org"), GrantFilter),
    security(("bearer" = [])),
    responses((status = 200, body = [GrantRecord]), (status = 403, body = ErrorBody)))]
pub(crate) async fn list_grants(
    State(state): State<AppState>,
    Path(org): Path<String>,
    Query(filter): Query<GrantFilter>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<GrantRecord>>, ApiError> {
    principal.require_admin(&org)?;
    let grants =
        blocking(move || Ok(state.store.list_grants(&org, filter.resource_ref.as_deref())?)).await?;
    Ok(Json(grants))
}

#[utoipa::path(post, path = "/api/v1/orgs/{org}/grants", request_body = CreateGrant, tag = "grants",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 201, body = GrantRecord), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_grant(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
    Json(req): Json<CreateGrant>,
) -> Result<(StatusCode, Json<GrantRecord>), ApiError> {
    principal.require_admin(&org)?;
    validate_subject_in_org(&state.store, &org, req.subject_kind, &req.subject_id)?;
    let grant = GrantRecord {
        id: Uuid::new_v4(),
        org,
        subject_kind: req.subject_kind,
        subject_id: req.subject_id,
        resource_kind: req.resource_kind,
        resource_ref: req.resource_ref,
        permission: req.permission,
        created_at: Utc::now(),
    };
    let stored = grant.clone();
    blocking(move || Ok(state.store.create_grant(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(grant)))
}

#[utoipa::path(delete, path = "/api/v1/orgs/{org}/grants/{id}", tag = "grants",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "Grant id")),
    security(("bearer" = [])),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_grant(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
) -> Result<StatusCode, ApiError> {
    principal.require_admin(&org)?;
    let existing = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_grant(id)?)).await?
    };
    if existing.org != org {
        return Err(ApiError::NotFound("grant"));
    }
    blocking(move || Ok(state.store.delete_grant(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/dashboards/{id}/grants", tag = "grants",
    params(("id" = String, Path, description = "Dashboard id")),
    security(("bearer" = [])),
    responses((status = 200, body = [GrantRecord]), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn list_dashboard_grants(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<GrantRecord>>, ApiError> {
    let dashboard = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_dashboard(id)?)).await?
    };
    principal.require_admin(&dashboard.org)?;
    let res_ref = resource_ref("dashboard", &id.to_string());
    let grants =
        blocking(move || Ok(state.store.list_grants(&dashboard.org, Some(&res_ref))?)).await?;
    Ok(Json(grants))
}

#[utoipa::path(post, path = "/api/v1/dashboards/{id}/grants", request_body = CreateDashboardGrant,
    tag = "grants", params(("id" = String, Path, description = "Dashboard id")),
    security(("bearer" = [])),
    responses((status = 201, body = GrantRecord), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_dashboard_grant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    principal: RequestPrincipal,
    Json(req): Json<CreateDashboardGrant>,
) -> Result<(StatusCode, Json<GrantRecord>), ApiError> {
    let dashboard = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_dashboard(id)?)).await?
    };
    principal.require_admin(&dashboard.org)?;
    validate_subject_in_org(&state.store, &dashboard.org, req.subject_kind, &req.subject_id)?;
    let grant = GrantRecord {
        id: Uuid::new_v4(),
        org: dashboard.org,
        subject_kind: req.subject_kind,
        subject_id: req.subject_id,
        resource_kind: "dashboard".into(),
        resource_ref: resource_ref("dashboard", &id.to_string()),
        permission: req.permission,
        created_at: Utc::now(),
    };
    let stored = grant.clone();
    blocking(move || Ok(state.store.create_grant(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(grant)))
}

/// A grant's subject must be a user/team that lives in the grant's org, so an
/// org-admin can't grant access to a subject from another tenant.
fn validate_subject_in_org(
    store: &crate::store::Store,
    org: &str,
    kind: SubjectKind,
    subject_id: &str,
) -> Result<(), ApiError> {
    let id = Uuid::parse_str(subject_id)
        .map_err(|_| ApiError::BadRequest("subject_id must be a uuid".into()))?;
    let subject_org = match kind {
        SubjectKind::User => store.get_user(id)?.org,
        SubjectKind::Team => store.get_team(id)?.org,
    };
    if subject_org != org {
        return Err(ApiError::BadRequest(
            "grant subject must belong to the org".into(),
        ));
    }
    Ok(())
}
