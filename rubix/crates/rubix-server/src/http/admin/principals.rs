//! `/principals` — CRUD over identities, gate-audited and admin-guarded.
//!
//! Surface 1 of `rubix/docs/design/ADMIN-API.md`. Users and extensions are one
//! identity model, so this is the single principal surface, distinguished only by
//! `kind`. Every route requires `Admin` in the caller's namespace (the transport
//! guard); mutations route through the gate's audited principal verbs so each
//! create/delete/role-change produces an immutable audit row with a correlation
//! id. The API subject is **local** to the namespace — a request subject `alice`
//! in `tenant_acme` is stored under the full subject `tenant_acme_alice`; the
//! prefix is applied here and stripped from every response.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use rubix_core::{Id, Principal, Role};
use rubix_gate::{
    create_principal, delete_principal, get_principal, list_principals, set_principal_role,
};

use crate::auth::Authenticated;
use crate::dto::admin::{
    CreatePrincipalRequest, CreatedPrincipalDto, PrincipalDto, UpdatePrincipalRequest, parse_kind,
    parse_role, prefix_subject,
};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::guard::require_admin;

/// `POST /principals` — provision a new identity in the caller's namespace.
///
/// The secret may be caller-supplied or omitted; when omitted the server mints a
/// random secret and returns it once in the response (ADMIN-API open item 5). A
/// supplied secret is never echoed back (the caller already holds it).
pub async fn create_principal_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<CreatePrincipalRequest>,
) -> ApiResult<(StatusCode, Json<CreatedPrincipalDto>)> {
    let namespace = require_admin(&auth.principal)?;
    let kind = parse_kind(&body.kind)
        .map_err(|k| ApiError::BadRequest(format!("unknown principal kind `{k}`")))?;
    let role =
        parse_role(&body.role).map_err(|r| ApiError::BadRequest(format!("unknown role `{r}`")))?;

    let full_subject = prefix_subject(&namespace, &body.subject);
    // Provision is non-idempotent; re-creating an existing subject is a conflict.
    if get_principal(state.store.raw(), &namespace, &full_subject)
        .await
        .map_err(map_admin_error)?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "a principal already exists under subject `{}`",
            body.subject
        )));
    }

    // Mint a secret when the caller did not supply one; the minted secret is the
    // only one ever returned on the wire (a supplied secret stays with the caller).
    let (secret, generated) = match body.secret {
        Some(s) => (s, None),
        None => {
            let minted = generate_secret();
            (minted.clone(), Some(minted))
        }
    };

    let principal = Principal::new(Id::from_raw(full_subject), namespace.clone(), kind, role);
    create_principal(state.store.raw(), &auth.principal, &principal, secret)
        .await
        .map_err(map_admin_error)?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedPrincipalDto::new(&principal, generated)),
    ))
}

/// Mint a random principal secret (a fresh UUID v4 — uncoordinated and unguessable
/// enough for a shared credential, the same primitive ids use).
fn generate_secret() -> String {
    Id::new().to_string()
}

/// `GET /principals` — list every identity in the caller's namespace.
pub async fn list_principals_route(
    State(state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<Vec<PrincipalDto>>> {
    let namespace = require_admin(&auth.principal)?;
    let principals = list_principals(state.store.raw(), &namespace)
        .await
        .map_err(map_admin_error)?;
    Ok(Json(
        principals
            .iter()
            .map(PrincipalDto::from_principal)
            .collect(),
    ))
}

/// `GET /principals/:subject` — fetch one identity, or `404`.
pub async fn get_principal_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
) -> ApiResult<Json<PrincipalDto>> {
    let namespace = require_admin(&auth.principal)?;
    let full_subject = prefix_subject(&namespace, &subject);
    let principal = get_principal(state.store.raw(), &namespace, &full_subject)
        .await
        .map_err(map_admin_error)?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(PrincipalDto::from_principal(&principal)))
}

/// `PATCH /principals/:subject` — change an identity's role (last-admin guarded).
pub async fn update_principal_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
    Json(body): Json<UpdatePrincipalRequest>,
) -> ApiResult<Json<PrincipalDto>> {
    let namespace = require_admin(&auth.principal)?;
    let role =
        parse_role(&body.role).map_err(|r| ApiError::BadRequest(format!("unknown role `{r}`")))?;
    let full_subject = prefix_subject(&namespace, &subject);

    let current = get_principal(state.store.raw(), &namespace, &full_subject)
        .await
        .map_err(map_admin_error)?
        .ok_or(ApiError::NotFound)?;

    // Demoting the final admin would lock the namespace out — refuse.
    if current.role == Role::Admin && role != Role::Admin {
        guard_not_last_admin(&state, &namespace).await?;
    }

    let updated = set_principal_role(
        state.store.raw(),
        &auth.principal,
        &namespace,
        &full_subject,
        role,
    )
    .await
    .map_err(map_admin_error)?;
    Ok(Json(PrincipalDto::from_principal(&updated)))
}

/// `DELETE /principals/:subject` — remove an identity (last-admin guarded).
pub async fn delete_principal_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(subject): Path<String>,
) -> ApiResult<StatusCode> {
    let namespace = require_admin(&auth.principal)?;
    let full_subject = prefix_subject(&namespace, &subject);

    let current = get_principal(state.store.raw(), &namespace, &full_subject)
        .await
        .map_err(map_admin_error)?
        .ok_or(ApiError::NotFound)?;

    if current.role == Role::Admin {
        guard_not_last_admin(&state, &namespace).await?;
    }

    delete_principal(
        state.store.raw(),
        &auth.principal,
        &namespace,
        &full_subject,
    )
    .await
    .map_err(map_admin_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Refuse the operation if the namespace has only one `Admin` left.
///
/// Self-lockout prevention (ADMIN-API, "Last-admin guard"): a namespace with zero
/// admins can only be recovered through the root/onboarding path, so demoting or
/// deleting the final admin is a `409 Conflict`. The caller invokes this only when
/// the target is itself an admin, so a total admin count of one means the target
/// is the last one.
async fn guard_not_last_admin(state: &AppState, namespace: &str) -> Result<(), ApiError> {
    let admins = list_principals(state.store.raw(), namespace)
        .await
        .map_err(map_admin_error)?
        .into_iter()
        .filter(|p| p.role == Role::Admin)
        .count();
    if admins <= 1 {
        return Err(ApiError::Conflict(
            "cannot remove the last admin in the namespace".to_owned(),
        ));
    }
    Ok(())
}

/// Map a gate failure from a principal verb to its transport status.
///
/// `Authenticate` is the gate's "unknown principal" signal (e.g. a role change on
/// a subject deleted concurrently), which maps to `404`. Existence and duplicate
/// cases are pre-checked in the handlers, so anything else is an internal failure.
pub(crate) fn map_admin_error(error: rubix_gate::GateError) -> ApiError {
    match error {
        rubix_gate::GateError::Authenticate(_) => ApiError::NotFound,
        other => ApiError::Internal(other.to_string()),
    }
}
