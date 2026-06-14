//! Org-scoped team management + memberships:
//! `/api/v1/orgs/{org}/teams` and `…/teams/{id}/members`. Admin-gated. See
//! `docs/design/authz-rbac.md` increment B/D.

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
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::store::{TeamRecord, UserRecord};
use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/orgs/{org}/teams",
            get(list_teams).post(create_team),
        )
        .route(
            "/api/v1/orgs/{org}/teams/{id}",
            get(get_team).patch(patch_team).delete(delete_team),
        )
        .route(
            "/api/v1/orgs/{org}/teams/{id}/members",
            get(list_members).post(add_member),
        )
        .route(
            "/api/v1/orgs/{org}/teams/{id}/members/{user_id}",
            axum::routing::delete(remove_member),
        )
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct CreateTeam {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PatchTeam {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct AddMember {
    pub user_id: Uuid,
}

#[utoipa::path(get, path = "/api/v1/orgs/{org}/teams", tag = "teams",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 200, body = [TeamRecord]), (status = 403, body = ErrorBody)))]
pub(crate) async fn list_teams(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<TeamRecord>>, ApiError> {
    principal.require_admin(&org)?;
    let teams = blocking(move || Ok(state.store.list_teams(&org)?)).await?;
    Ok(Json(teams))
}

#[utoipa::path(post, path = "/api/v1/orgs/{org}/teams", request_body = CreateTeam, tag = "teams",
    params(("org" = String, Path, description = "Tenant org")),
    security(("bearer" = [])),
    responses((status = 201, body = TeamRecord), (status = 400, body = ErrorBody),
              (status = 403, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub(crate) async fn create_team(
    State(state): State<AppState>,
    Path(org): Path<String>,
    principal: RequestPrincipal,
    Json(req): Json<CreateTeam>,
) -> Result<(StatusCode, Json<TeamRecord>), ApiError> {
    validate_slug(&org)?;
    validate_slug(&req.slug)?;
    principal.require_admin(&org)?;
    let team = TeamRecord {
        id: Uuid::new_v4(),
        org,
        slug: req.slug,
        name: req.name,
        created_at: Utc::now(),
    };
    let stored = team.clone();
    blocking(move || Ok(state.store.create_team(&stored)?)).await?;
    Ok((StatusCode::CREATED, Json(team)))
}

#[utoipa::path(get, path = "/api/v1/orgs/{org}/teams/{id}", tag = "teams",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "Team id")),
    security(("bearer" = [])),
    responses((status = 200, body = TeamRecord), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn get_team(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
) -> Result<Json<TeamRecord>, ApiError> {
    principal.require_admin(&org)?;
    let team = blocking(move || Ok(state.store.get_team(id)?)).await?;
    guard_same_org(&team.org, &org)?;
    Ok(Json(team))
}

#[utoipa::path(patch, path = "/api/v1/orgs/{org}/teams/{id}", request_body = PatchTeam, tag = "teams",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "Team id")),
    security(("bearer" = [])),
    responses((status = 200, body = TeamRecord), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn patch_team(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
    Json(req): Json<PatchTeam>,
) -> Result<Json<TeamRecord>, ApiError> {
    principal.require_admin(&org)?;
    let existing = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_team(id)?)).await?
    };
    guard_same_org(&existing.org, &org)?;
    let team = blocking(move || Ok(state.store.update_team(id, req.name.as_deref())?)).await?;
    Ok(Json(team))
}

#[utoipa::path(delete, path = "/api/v1/orgs/{org}/teams/{id}", tag = "teams",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "Team id")),
    security(("bearer" = [])),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn delete_team(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
) -> Result<StatusCode, ApiError> {
    principal.require_admin(&org)?;
    let existing = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_team(id)?)).await?
    };
    guard_same_org(&existing.org, &org)?;
    blocking(move || Ok(state.store.delete_team(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/orgs/{org}/teams/{id}/members", tag = "teams",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "Team id")),
    security(("bearer" = [])),
    responses((status = 200, body = [UserRecord]), (status = 403, body = ErrorBody),
              (status = 404, body = ErrorBody)))]
pub(crate) async fn list_members(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<UserRecord>>, ApiError> {
    principal.require_admin(&org)?;
    let team = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_team(id)?)).await?
    };
    guard_same_org(&team.org, &org)?;
    let members = blocking(move || Ok(state.store.list_team_members(id)?)).await?;
    Ok(Json(members))
}

#[utoipa::path(post, path = "/api/v1/orgs/{org}/teams/{id}/members", request_body = AddMember, tag = "teams",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "Team id")),
    security(("bearer" = [])),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn add_member(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, Uuid)>,
    principal: RequestPrincipal,
    Json(req): Json<AddMember>,
) -> Result<StatusCode, ApiError> {
    principal.require_admin(&org)?;
    // Team and user must both live in the path org (cross-tenant denial).
    let (team, user) = {
        let store = state.store.clone();
        let user_id = req.user_id;
        blocking(move || Ok((store.get_team(id)?, store.get_user(user_id)?))).await?
    };
    guard_same_org(&team.org, &org)?;
    guard_same_org(&user.org, &org)?;
    blocking(move || Ok(state.store.add_team_member(id, req.user_id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/api/v1/orgs/{org}/teams/{id}/members/{user_id}", tag = "teams",
    params(("org" = String, Path, description = "Tenant org"),
           ("id" = String, Path, description = "Team id"),
           ("user_id" = String, Path, description = "User id")),
    security(("bearer" = [])),
    responses((status = 204), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub(crate) async fn remove_member(
    State(state): State<AppState>,
    Path((org, id, user_id)): Path<(String, Uuid, Uuid)>,
    principal: RequestPrincipal,
) -> Result<StatusCode, ApiError> {
    principal.require_admin(&org)?;
    let team = {
        let store = state.store.clone();
        blocking(move || Ok(store.get_team(id)?)).await?
    };
    guard_same_org(&team.org, &org)?;
    blocking(move || Ok(state.store.remove_team_member(id, user_id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn guard_same_org(resource_org: &str, path_org: &str) -> Result<(), ApiError> {
    if resource_org != path_org {
        return Err(ApiError::NotFound("team"));
    }
    Ok(())
}
