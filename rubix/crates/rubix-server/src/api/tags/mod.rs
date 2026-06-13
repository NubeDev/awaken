//! Entity-tag routes (docs/design/page-context-and-nav.md §3). Tags are
//! behaviour-affecting (a dashboard's tags feed `PageContext.tags` and drive
//! queries), so the write/read handlers resolve the target entity and enforce
//! **the entity's own authz** — `edit` to write, `view` to read — and reject
//! unknown/foreign ids. Wiring only; one file per verb.

pub(crate) mod entities;
pub(crate) mod get;
pub(crate) mod keys;
pub(crate) mod put;

use axum::routing::get;
use axum::Router;
use rubix_core::TagEntityKind;
use uuid::Uuid;

use crate::api::dashboards::{authorize_dashboard_write_existing, may_read_dashboard};
use crate::auth::RequestPrincipal;
use crate::error::ApiError;
use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/tags/keys", get(keys::tag_keys))
        .route("/api/v1/tags/entities/{kind}", get(entities::tagged_entities))
        .route(
            "/api/v1/tags/{kind}/{id}",
            get(get::get_tags).put(put::put_tags),
        )
}

/// Parse the `{kind}` path segment into a known [`TagEntityKind`]; an unknown
/// kind is a 404 (the entity class does not exist as a tag surface).
pub(crate) fn parse_kind(raw: &str) -> Result<TagEntityKind, ApiError> {
    TagEntityKind::parse(raw).ok_or(ApiError::NotFound("tag entity kind"))
}

/// Resolve the target entity, confirm it lives in the caller-visible org, and
/// enforce **its own** read authz. Returns the entity's org (the tag rows' tenant
/// key). Rejects an unknown/foreign id as a 404 by construction — the entity's
/// own `get` is the existence oracle, and the read gate hides what the caller may
/// not see.
pub(crate) async fn authorize_entity_read(
    state: &AppState,
    principal: &RequestPrincipal,
    kind: TagEntityKind,
    id: Uuid,
) -> Result<String, ApiError> {
    match kind {
        TagEntityKind::Dashboard => {
            let store = state.store.clone();
            let dashboard =
                crate::api::blocking::blocking(move || Ok(store.get_dashboard(id)?)).await?;
            if !may_read_dashboard(principal, &state.store, &dashboard) {
                return Err(ApiError::NotFound("dashboard"));
            }
            Ok(dashboard.org)
        }
    }
}

/// Resolve the target entity and enforce **its own** write (`edit`) authz, the
/// rule for mutating its tags. Returns the entity's org.
pub(crate) async fn authorize_entity_write(
    state: &AppState,
    principal: &RequestPrincipal,
    kind: TagEntityKind,
    id: Uuid,
) -> Result<String, ApiError> {
    match kind {
        TagEntityKind::Dashboard => {
            let store = state.store.clone();
            let dashboard =
                crate::api::blocking::blocking(move || Ok(store.get_dashboard(id)?)).await?;
            authorize_dashboard_write_existing(
                principal,
                &state.store,
                &dashboard.org,
                dashboard.site_id,
                id,
            )?;
            Ok(dashboard.org)
        }
    }
}

/// The org a tag-key/reverse-lookup read is scoped to. These routes are not
/// entity-addressed, so they gate on the caller's org scope directly. The org is
/// taken from the principal's scope; an unauthenticated edge caller (no
/// principal) falls back to a required `org` query param.
pub(crate) fn require_read_scope_org(
    principal: &RequestPrincipal,
    org: &str,
) -> Result<(), ApiError> {
    principal.authorize_read(&crate::auth::Scope::org(org))?;
    Ok(())
}
