//! `/tenants` — cloud onboarding of namespaces, root/system authorized.
//!
//! Surface 3 of `rubix/docs/design/ADMIN-API.md`. "Create a tenant" means
//! bootstrap a namespace (seed its meta-collection + gate/audit schema, provision
//! its first admin) plus write one lightweight registry record so the namespace is
//! discoverable. A fresh namespace has no admin, so the admin-in-namespace rule
//! cannot apply: these routes are authorized for a **root/system principal**.
//!
//! One binary, edge to cloud: the routes are always mounted, never `#[cfg]`-gated,
//! so the route table is identical on every build. Handlers branch on
//! `state.profile.is_multi_tenant()`; on edge, mutation returns `409 Conflict`.
//!
//! **System principal (ADMIN-API Open item 2, resolved here).** A request is
//! root/system when its principal is an `Admin` in the server's configured root
//! namespace (`state.namespace`) — the deployment's own bootstrap identity, the
//! one namespace that exists before any tenant. This is the single recognized
//! root path; cross-tenant admin does not flow through header auth.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{create_principal, purge_namespace};

use crate::auth::Authenticated;
use crate::dto::admin::{CreateTenantRequest, TenantDto, prefix_subject};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::tenants::{
    StoredTenant, create_tenant, delete_tenant, get_tenant, list_tenants,
};

/// `POST /tenants` — onboard a new namespace (cloud only).
pub async fn create_tenant_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Json(body): Json<CreateTenantRequest>,
) -> ApiResult<(StatusCode, Json<TenantDto>)> {
    require_system(&state, &auth.principal)?;
    require_multi_tenant(&state)?;

    let id = sanitize_id(&body.id)?;
    if get_tenant(state.store.raw(), &id)
        .await
        .map_err(ApiError::Internal)?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "a tenant is already onboarded under id `{id}`"
        )));
    }

    let namespace = state
        .profile
        .resolve_namespace(&state.namespace, Some(&id))
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Bootstrap the namespace: meta-collection so it exposes a collection registry
    // from the first read. The gate/audit schema is store-wide (defined at boot),
    // so only the per-namespace meta-collection is seeded here.
    rubix_core::bootstrap_meta_collection(state.store.raw(), &namespace)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Provision the tenant's first admin — the identity the admin-in-namespace
    // rule will recognize for every subsequent `/principals` call in this tenant.
    let admin_subject = prefix_subject(&namespace, &body.admin_subject);
    let admin = Principal::new(
        Id::from_raw(admin_subject.clone()),
        namespace.clone(),
        PrincipalKind::User,
        Role::Admin,
    );
    create_principal(state.store.raw(), &auth.principal, &admin, body.admin_secret)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let now = Utc::now();
    let tenant = StoredTenant {
        id: id.clone(),
        namespace: namespace.clone(),
        created_at: now,
        first_admin_subject: admin_subject,
    };
    create_tenant(state.store.raw(), &tenant)
        .await
        .map_err(ApiError::Internal)?;

    Ok((StatusCode::CREATED, Json(tenant_dto(&tenant))))
}

/// `GET /tenants` — list onboarded tenants (cloud) or the single namespace (edge).
pub async fn list_tenants_route(
    State(state): State<AppState>,
    auth: Authenticated,
) -> ApiResult<Json<Vec<TenantDto>>> {
    require_system(&state, &auth.principal)?;

    if !state.profile.is_multi_tenant() {
        // Edge is single-namespace: report the one configured namespace so the
        // surface is uniform across builds, without a registry it never writes.
        return Ok(Json(vec![TenantDto {
            id: state.namespace.clone(),
            namespace: state.namespace.clone(),
            created_at: String::new(),
        }]));
    }

    let tenants = list_tenants(state.store.raw())
        .await
        .map_err(ApiError::Internal)?;
    Ok(Json(tenants.iter().map(tenant_dto).collect()))
}

/// Project a stored tenant into its wire DTO.
fn tenant_dto(tenant: &StoredTenant) -> TenantDto {
    TenantDto {
        id: tenant.id.clone(),
        namespace: tenant.namespace.clone(),
        created_at: tenant.created_at.to_rfc3339(),
    }
}

/// Authorize a root/system principal — an `Admin` in the server's root namespace.
fn require_system(state: &AppState, principal: &Principal) -> Result<(), ApiError> {
    if principal.role == Role::Admin && principal.namespace == state.namespace {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "tenant onboarding requires a root/system principal".to_owned(),
        ))
    }
}

/// Reject tenant mutation on a single-namespace (edge) profile with `409`.
fn require_multi_tenant(state: &AppState) -> Result<(), ApiError> {
    if state.profile.is_multi_tenant() {
        Ok(())
    } else {
        Err(ApiError::Conflict(
            "tenant onboarding is a cloud action; this node runs a single namespace".to_owned(),
        ))
    }
}

/// Validate the tenant id is a safe namespace suffix (alphanumeric, `-`, `_`).
///
/// The id becomes part of a namespace name and a record key, so it is constrained
/// to a conservative character set rather than trusting arbitrary input.
fn sanitize_id(raw: &str) -> Result<String, ApiError> {
    let id = raw.trim();
    if id.is_empty() {
        return Err(ApiError::BadRequest("tenant id must not be empty".to_owned()));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ApiError::BadRequest(
            "tenant id must be alphanumeric with `-` or `_` only".to_owned(),
        ));
    }
    Ok(id.to_owned())
}

/// `DELETE /tenants/:id` — drop a tenant namespace (cloud only, root-authorized).
///
/// Dropping a tenant is irreversible (ADMIN-API Open item 1): it purges every
/// gate-owned row tagged with the tenant namespace — records, principals, grants —
/// then deletes the registry record. Gated behind a root/system principal and the
/// multi-tenant profile, with an explicit `?confirm={id}` guard so the
/// irreversible action cannot fire by accident. The route is always mounted (one
/// binary edge-to-cloud); on edge it returns `409`.
pub async fn delete_tenant_route(
    State(state): State<AppState>,
    auth: Authenticated,
    Path(id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<DeleteTenantQuery>,
) -> ApiResult<StatusCode> {
    require_system(&state, &auth.principal)?;
    require_multi_tenant(&state)?;

    // Explicit confirmation: the caller must echo the tenant id to proceed.
    if query.confirm.as_deref() != Some(id.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "tenant deletion is irreversible; resend with `?confirm={id}` to proceed"
        )));
    }

    let tenant = get_tenant(state.store.raw(), &id)
        .await
        .map_err(ApiError::Internal)?
        .ok_or(ApiError::NotFound)?;

    // Purge the namespace's gate-owned rows (audited), then drop the registry row.
    purge_namespace(state.store.raw(), &auth.principal, &tenant.namespace)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    delete_tenant(state.store.raw(), &id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Query parameters for tenant deletion — the confirmation guard.
#[derive(Debug, serde::Deserialize)]
pub struct DeleteTenantQuery {
    /// The tenant id the caller re-states to confirm the irreversible delete.
    confirm: Option<String>,
}
