//! `/principals/:subject/grants` — capability grants nested under a principal.
//!
//! Surface 2 of `rubix/docs/design/ADMIN-API.md`. A grant has no identity apart
//! from `(namespace, subject, capability)`, so it is addressed under its
//! principal. The handler **loads the principal first** (the gate's `create_grant`
//! does not verify the grantee exists), returning `404` for an unknown subject
//! before granting — no orphan grants. Mutations route through the gate's audited
//! grant verbs; the gate's own `may_administer` check enforces admin-in-namespace,
//! and a denial surfaces as `403`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rubix_gate::{
    Capability, create_grant_audited, create_team_grant_audited, get_principal, get_team,
    list_grants, list_team_grants, revoke_grant_audited, revoke_team_grant_audited,
};

use crate::auth::Authenticated;
use crate::dto::admin::{GrantDto, prefix_subject, strip_subject_prefix};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::guard::require_admin;

/// `GET /principals/:subject/grants` — list a principal's capability grants.
pub async fn list_grants_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
) -> ApiResult<Json<Vec<GrantDto>>> {
    let namespace = require_admin(&auth.principal)?;
    let principal = load_principal(&state, &namespace, &subject).await?;
    let grants = list_grants(state.store.raw(), &principal)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(
        grants
            .iter()
            .map(|g| GrantDto {
                subject: strip_subject_prefix(&g.namespace, &g.subject),
                namespace: g.namespace.clone(),
                capability: g.capability.as_str().to_owned(),
            })
            .collect(),
    ))
}

/// `PUT /principals/:subject/grants/:capability` — grant a capability (idempotent).
pub async fn put_grant_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path((subject, capability)): Path<(String, String)>,
) -> ApiResult<Json<GrantDto>> {
    let namespace = require_admin(&auth.principal)?;
    let capability = parse_capability(&capability)?;
    let principal = load_principal(&state, &namespace, &subject).await?;

    let grant = create_grant_audited(state.store.raw(), &auth.principal, &principal, capability)
        .await
        .map_err(map_grant_error)?;
    Ok(Json(GrantDto {
        subject: strip_subject_prefix(&grant.namespace, &grant.subject),
        namespace: grant.namespace,
        capability: grant.capability.as_str().to_owned(),
    }))
}

/// `DELETE /principals/:subject/grants/:capability` — revoke (idempotent).
pub async fn delete_grant_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path((subject, capability)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let namespace = require_admin(&auth.principal)?;
    let capability = parse_capability(&capability)?;
    let principal = load_principal(&state, &namespace, &subject).await?;

    revoke_grant_audited(state.store.raw(), &auth.principal, &principal, capability)
        .await
        .map_err(map_grant_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /teams/:slug/grants` — list a team's capability grants.
///
/// These are the grants every member of the team inherits (`rubix/docs/SCOPE.md`,
/// team inheritance). The capability check resolves them per-member; this surface
/// lets an admin see and manage them on the team itself.
pub async fn list_team_grants_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(slug): Path<String>,
) -> ApiResult<Json<Vec<GrantDto>>> {
    let namespace = require_admin(&auth.principal)?;
    require_team(&state, &namespace, &slug).await?;
    let grants = list_team_grants(state.store.raw(), &namespace, &slug)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(
        grants
            .iter()
            .map(|g| GrantDto {
                // A team grant's subject is the `team:{slug}` form — surface it as-is
                // so the client can tell a team grant from a principal grant.
                subject: g.subject.clone(),
                namespace: g.namespace.clone(),
                capability: g.capability.as_str().to_owned(),
            })
            .collect(),
    ))
}

/// `PUT /teams/:slug/grants/:capability` — grant a capability to a team (idempotent).
pub async fn put_team_grant_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path((slug, capability)): Path<(String, String)>,
) -> ApiResult<Json<GrantDto>> {
    let namespace = require_admin(&auth.principal)?;
    let capability = parse_capability(&capability)?;
    require_team(&state, &namespace, &slug).await?;

    let grant = create_team_grant_audited(
        state.store.raw(),
        &auth.principal,
        &slug,
        &namespace,
        capability,
    )
    .await
    .map_err(map_grant_error)?;
    Ok(Json(GrantDto {
        subject: grant.subject,
        namespace: grant.namespace,
        capability: grant.capability.as_str().to_owned(),
    }))
}

/// `DELETE /teams/:slug/grants/:capability` — revoke a team capability (idempotent).
pub async fn delete_team_grant_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path((slug, capability)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let namespace = require_admin(&auth.principal)?;
    let capability = parse_capability(&capability)?;
    require_team(&state, &namespace, &slug).await?;

    revoke_team_grant_audited(
        state.store.raw(),
        &auth.principal,
        &slug,
        &namespace,
        capability,
    )
    .await
    .map_err(map_grant_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Resolve a team by slug, `404` if it does not exist in `namespace` — the same
/// existence guard the principal grant routes apply to their subject.
async fn require_team(state: &AppState, namespace: &str, slug: &str) -> Result<(), ApiError> {
    get_team(state.store.raw(), namespace, slug)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)
        .map(|_| ())
}

/// Load the grant target by its API-local subject, `404` if it does not exist.
///
/// `create_grant` accepts any `Principal` and does not check existence, so the
/// route resolves the principal first to avoid creating an orphan grant.
async fn load_principal(
    state: &AppState,
    namespace: &str,
    subject: &str,
) -> Result<rubix_core::Principal, ApiError> {
    let full_subject = prefix_subject(namespace, subject);
    get_principal(state.store.raw(), namespace, &full_subject)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or(ApiError::NotFound)
}

/// Parse a capability path segment, `400` for an unknown capability.
fn parse_capability(raw: &str) -> Result<Capability, ApiError> {
    Capability::parse(raw)
        .ok_or_else(|| ApiError::BadRequest(format!("unknown capability `{raw}`")))
}

/// Map a gate grant failure to its transport status — a denial is `403`.
fn map_grant_error(error: rubix_gate::GateError) -> ApiError {
    match error {
        rubix_gate::GateError::GrantDenied(reason) => ApiError::Forbidden(reason),
        other => ApiError::Internal(other.to_string()),
    }
}
