//! `/teams` — CRUD over teams and their memberships, gate-audited, admin-guarded.
//!
//! A team is a named group of principals; granting a capability (or, later, a
//! nav node) to a team flows it to every member (`rubix/docs/SCOPE.md`,
//! principle 5; team inheritance in `rubix-gate`). Every route requires `Admin`
//! in the caller's namespace (the transport guard), and mutations route through
//! the gate's audited team verbs. Team **slugs** are API-local and namespace-
//! scoped; member **subjects** follow the principal convention — the API takes
//! the local subject (`alice`) and stores the prefixed `{namespace}_alice`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rubix_gate::{
    Team, add_member, create_team, delete_team, get_principal, get_team, list_members, list_teams,
    remove_member,
};

use crate::auth::Authenticated;
use crate::dto::admin::{
    AddMemberRequest, CreateTeamRequest, TeamDto, TeamMemberDto, prefix_subject,
    strip_subject_prefix,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::guard::require_admin;
use super::principals::map_admin_error;

/// `POST /teams` — create a team in the caller's namespace.
///
/// Idempotent on the slug (re-creating updates the display name). The slug is
/// trimmed; an empty slug is a `400`.
pub async fn create_team_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<CreateTeamRequest>,
) -> ApiResult<(StatusCode, Json<TeamDto>)> {
    let namespace = require_admin(&auth.principal)?;
    let slug = body.slug.trim();
    if slug.is_empty() {
        return Err(ApiError::BadRequest(
            "team slug must not be empty".to_owned(),
        ));
    }
    let display_name = body
        .display_name
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| slug.to_owned());

    let team = Team::new(slug, namespace.clone(), display_name);
    let created = create_team(state.store.raw(), &auth.principal, &team)
        .await
        .map_err(map_admin_error)?;
    Ok((StatusCode::CREATED, Json(TeamDto::from_team(&created))))
}

/// `GET /teams` — list every team in the caller's namespace.
pub async fn list_teams_route(
    State(state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<Vec<TeamDto>>> {
    let namespace = require_admin(&auth.principal)?;
    let teams = list_teams(state.store.raw(), &namespace)
        .await
        .map_err(map_admin_error)?;
    Ok(Json(teams.iter().map(TeamDto::from_team).collect()))
}

/// `GET /teams/:slug` — fetch one team, or `404`.
pub async fn get_team_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(slug): Path<String>,
) -> ApiResult<Json<TeamDto>> {
    let namespace = require_admin(&auth.principal)?;
    let team = get_team(state.store.raw(), &namespace, &slug)
        .await
        .map_err(map_admin_error)?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(TeamDto::from_team(&team)))
}

/// `DELETE /teams/:slug` — remove a team (and its memberships), `404` if absent.
pub async fn delete_team_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    let namespace = require_admin(&auth.principal)?;
    // Resolve first so an unknown slug is a 404 rather than a silently-audited no-op.
    get_team(state.store.raw(), &namespace, &slug)
        .await
        .map_err(map_admin_error)?
        .ok_or(ApiError::NotFound)?;
    delete_team(state.store.raw(), &auth.principal, &namespace, &slug)
        .await
        .map_err(map_admin_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /teams/:slug/members` — list a team's members (API-local subjects).
pub async fn list_members_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(slug): Path<String>,
) -> ApiResult<Json<Vec<TeamMemberDto>>> {
    let namespace = require_admin(&auth.principal)?;
    require_team(&state, &namespace, &slug).await?;
    let members = list_members(state.store.raw(), &namespace, &slug)
        .await
        .map_err(map_admin_error)?;
    Ok(Json(
        members
            .iter()
            .map(|full| TeamMemberDto {
                subject: strip_subject_prefix(&namespace, full),
            })
            .collect(),
    ))
}

/// `POST /teams/:slug/members` — add a principal to a team.
///
/// Resolves both the team and the principal first (no membership pointing at a
/// team or a subject that does not exist), then writes the link. Idempotent.
pub async fn add_member_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(slug): Path<String>,
    Json(body): Json<AddMemberRequest>,
) -> ApiResult<(StatusCode, Json<TeamMemberDto>)> {
    let namespace = require_admin(&auth.principal)?;
    require_team(&state, &namespace, &slug).await?;
    let full_subject = prefix_subject(&namespace, &body.subject);
    // The principal must exist in the namespace, or it is a 404 (no orphan member).
    get_principal(state.store.raw(), &namespace, &full_subject)
        .await
        .map_err(map_admin_error)?
        .ok_or(ApiError::NotFound)?;

    add_member(
        state.store.raw(),
        &auth.principal,
        &namespace,
        &slug,
        &full_subject,
    )
    .await
    .map_err(map_admin_error)?;
    Ok((
        StatusCode::CREATED,
        Json(TeamMemberDto {
            subject: body.subject,
        }),
    ))
}

/// `DELETE /teams/:slug/members/:subject` — remove a principal from a team (idempotent).
pub async fn remove_member_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path((slug, subject)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let namespace = require_admin(&auth.principal)?;
    require_team(&state, &namespace, &slug).await?;
    let full_subject = prefix_subject(&namespace, &subject);
    remove_member(
        state.store.raw(),
        &auth.principal,
        &namespace,
        &slug,
        &full_subject,
    )
    .await
    .map_err(map_admin_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Resolve a team by slug, returning `404` if it does not exist in `namespace`.
///
/// The membership routes nest under a team, so they confirm the team is real
/// before touching memberships — the same guard the grant routes apply to a
/// principal.
async fn require_team(state: &AppState, namespace: &str, slug: &str) -> Result<(), ApiError> {
    get_team(state.store.raw(), namespace, slug)
        .await
        .map_err(map_admin_error)?
        .ok_or(ApiError::NotFound)
        .map(|_| ())
}
