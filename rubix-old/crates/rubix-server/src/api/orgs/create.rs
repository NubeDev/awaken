//! POST /api/v1/orgs — provision a tenant in one call: create its first site
//! under a new (or existing) org. There is no separate org row to create, so
//! "onboard KFC" is creating the site `kfc/<slug>`; this endpoint is the
//! single-action convenience the admin UI calls instead of POSTing a bare site.
//! Token minting for the new tenant stays a separate `POST /tokens` step.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use rubix_core::{validate_slug, Site, TagSet};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::list::OrgSummary;
use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

/// Provision a new tenant: its org plus the first site under it.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProvisionOrg {
    /// The org namespace (the tenant key); becomes `site.org`.
    pub org: String,
    /// Slug of the first site under the org.
    pub slug: String,
    pub display_name: String,
    #[serde(default)]
    pub tags: TagSet,
}

#[utoipa::path(post, path = "/api/v1/orgs", request_body = ProvisionOrg, tag = "orgs",
    security(("bearer" = [])),
    responses((status = 201, body = OrgSummary), (status = 400, body = ErrorBody),
              (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
              (status = 409, body = ErrorBody)))]
pub(crate) async fn provision_org(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<ProvisionOrg>,
) -> Result<(StatusCode, Json<OrgSummary>), ApiError> {
    validate_slug(&req.org)?;
    validate_slug(&req.slug)?;
    req.tags.validate()?;
    principal.authorize_site_write(&req.org, &req.slug)?;

    let site = Site {
        id: Uuid::new_v4(),
        org: req.org.clone(),
        slug: req.slug.clone(),
        display_name: req.display_name,
        tags: req.tags.clone(),
        created_at: Utc::now(),
    };
    let org = req.org.clone();
    let stored = site.clone();
    let seed_org = req.org.clone();
    blocking(move || {
        state.store.create_site(&stored)?;
        // Seed the default nav tree the first time an org is provisioned, so every
        // built-in static page is a granted-org-wide node and later gating is
        // opt-in tightening (docs/design/page-context-and-nav.md §6). Re-provision
        // (the org already has nodes) skips it.
        if state.store.list_nav_nodes(&seed_org)?.is_empty() {
            crate::api::nav::seed_default_tree(&state.store, &seed_org)?;
        }
        Ok(())
    })
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(OrgSummary {
            org,
            site_count: 1,
            sites: vec![req.slug],
            tags: req.tags,
        }),
    ))
}
