//! Org-scoped user management: `/api/v1/orgs/{org}/users`. All mutations require
//! an admin covering the org (see [`RequestPrincipal::require_admin`]); the list
//! and get require an admin too, since the member roster is an admin surface.
//! See `docs/design/authz-rbac.md` increment B/D.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use rubix_core::validate_slug;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::api::blocking::blocking;
use crate::auth::{AdminLevel, RequestPrincipal};
use crate::error::{ApiError, ErrorBody};
use crate::store::UserRecord;
use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/orgs/{org}/users",
            get(list_users).post(create_user),
        )
        .route(
            "/api/v1/orgs/{org}/users/{id}",
            get(get_user).patch(patch_user).delete(delete_user),
        )
}

/// Body for creating a user. `admin_level` defaults to `none`.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateUser {
    /// The verified token subject (OIDC `sub` / PAT id) this user is keyed by.
    pub subject: String,
    pub email: String,
    pub display_name: String,
    #[serde(default)]
    #[schema(value_type = String)]
    pub admin_level: AdminLevel,
}

/// Partial update; absent fields are unchanged. `org`/`subject` are immutable.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PatchUser {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub admin_level: Option<AdminLevel>,
}

#[utoipa::path(get, path = "/api/v1/orgs/{org}/users", tag = "users",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 200, body = [UserRecord]), (status = 403, body = ErrorBody)))]
pub(crate) async fn list_users(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<UserRecord>>, ApiError> {
    principal.require_admin(&org)?;
    let users = blocking(move || Ok(state.store.list_users(&org)?)).await?;
    Ok(Json(users))
}

#[utoipa::path(post, path = "/api/v1/orgs/{org}/users", request_body = CreateUser, tag = "users",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 201, body = UserRecord), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_user(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
    Json(req): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserRecord>), ApiError> {
    validate_slug(&org)?;
    let admin = principal.require_admin(&org)?;
    // Only a super-admin may mint another super-admin; an org-admin caps at
    // org-admin (no privilege escalation past one's own tier).
    if req.admin_level == AdminLevel::SuperAdmin && !admin.scope.is_global() {
        return Err(ApiError::Forbidden(
            "only a super-admin may grant super-admin".into(),
        ));
    }
    if req.email.trim().is_empty() || req.subject.trim().is_empty() {
        return Err(ApiError::BadRequest("subject and email are required".into()));
    }
    let user = UserRecord {
        id: Uuid::new_v4(),
        org,
        subject: req.subject,
        email: req.email,
        display_name: req.display_name,
        admin_level: req.admin_level,
        created_at: Utc::now(),
    };
    let stored = user.clone();
    blocking(move || Ok(state.store.create_user(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

#[utoipa::path(get, path = "/api/v1/orgs/{org}/users/{id}", tag = "users",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "User id")),
    security(("bearer" = [])),
    responses((status = 200, body = UserRecord), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn get_user(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
) -> Result<Json<UserRecord>, ApiError> {
    principal.require_admin(&org)?;
    let user = blocking(move || Ok(state.store.get_user(id)?)).await?;
    guard_same_org(&user.org, &org)?;
    Ok(Json(user))
}

#[utoipa::path(patch, path = "/api/v1/orgs/{org}/users/{id}", request_body = PatchUser, tag = "users",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "User id")),
    security(("bearer" = [])),
    responses((status = 200, body = UserRecord), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_user(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
    Json(req): Json<PatchUser>,
) -> Result<Json<UserRecord>, ApiError> {
    let admin = principal.require_admin(&org)?;
    if req.admin_level == Some(AdminLevel::SuperAdmin) && !admin.scope.is_global() {
        return Err(ApiError::Forbidden(
            "only a super-admin may grant super-admin".into(),
        ));
    }
    let existing = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_user(id)?)).await?
    };
    guard_same_org(&existing.org, &org)?;
    let user = blocking(move || {
        Ok(state.store.update_user(
            id,
            req.email.as_deref(),
            req.display_name.as_deref(),
            req.admin_level,
        )?)
    })
    .await?;
    Ok(Json(user))
}

#[utoipa::path(delete, path = "/api/v1/orgs/{org}/users/{id}", tag = "users",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "User id")),
    security(("bearer" = [])),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_user(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
) -> Result<StatusCode, ApiError> {
    principal.require_admin(&org)?;
    let existing = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_user(id)?)).await?
    };
    guard_same_org(&existing.org, &org)?;
    blocking(move || Ok(state.store.delete_user(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// A by-id resource must live in the path's org, so an org-admin of A can never
/// reach a row in org B by guessing its id (cross-tenant denial). A super-admin
/// passes `require_admin` for any org, so this is the only tenant check for them.
fn guard_same_org(resource_org: &str, path_org: &str) -> Result<(), ApiError> {
    if resource_org != path_org {
        return Err(ApiError::NotFound("user"));
    }
    Ok(())
}
