//! GET /api/v1/orgs — the tenant list, derived by grouping the sites the
//! principal may see by their `org` string. Each entry carries the site count
//! and the union of site tags, so the admin UI can render a tenant table
//! without a dedicated org entity.

use std::collections::BTreeMap;

use axum::extract::{Query, State};
use axum::Json;
use rubix_core::TagSet;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::api::blocking::blocking;
use crate::auth::RequestPrincipal;
use crate::error::{ApiError, ErrorBody};
use crate::AppState;

/// A derived tenant: one distinct `org` with the sites visible under it.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrgSummary {
    /// The org namespace string (the tenant key).
    pub org: String,
    /// Number of visible sites under this org.
    pub site_count: usize,
    /// Slugs of the visible sites, sorted — the tenant's site list.
    pub sites: Vec<String>,
    /// Union of all site tags under this org (Haystack markers).
    pub tags: TagSet,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListOrgsQuery {
    /// Restrict to one org (the same filter `GET /sites?org=` accepts).
    pub org: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/orgs", params(ListOrgsQuery), tag = "orgs",
    security(("bearer" = [])),
    responses((status = 200, body = [OrgSummary]), (status = 401, body = ErrorBody)))]
pub(crate) async fn list_orgs(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(q): Query<ListOrgsQuery>,
) -> Result<Json<Vec<OrgSummary>>, ApiError> {
    let sites = blocking(move || Ok(state.store.list_sites(q.org.as_deref())?)).await?;

    // Group the visible sites by org. The visibility filter mirrors `list_sites`
    // so a scoped caller never learns about an org outside its scope.
    let mut by_org: BTreeMap<String, OrgSummary> = BTreeMap::new();
    for site in sites {
        if principal
            .authorize_site_read(&site.org, &site.slug)
            .is_err()
        {
            continue;
        }
        let entry = by_org
            .entry(site.org.clone())
            .or_insert_with(|| OrgSummary {
                org: site.org.clone(),
                site_count: 0,
                sites: Vec::new(),
                tags: TagSet::default(),
            });
        entry.site_count += 1;
        entry.sites.push(site.slug);
        for (name, value) in &site.tags.0 {
            entry
                .tags
                .0
                .entry(name.clone())
                .or_insert_with(|| value.clone());
        }
    }

    Ok(Json(by_org.into_values().collect()))
}
