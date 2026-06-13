//! Dashboard routes — named boards of widgets, scoped to an org (overview) or a
//! single site. Wiring only; one file per verb.
//!
//! Auth: a site-scoped board gates on `authorize_site_{read,write}(org, slug)`;
//! an org overview gates on the org scope directly. Reads a caller may not see
//! are filtered before the wire, exactly like sites.

pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod get;
pub(crate) mod list;
pub(crate) mod patch;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/dashboards",
            post(create::create_dashboard).get(list::list_dashboards),
        )
        .route(
            "/api/v1/dashboards/{id}",
            get(get::get_dashboard)
                .patch(patch::patch_dashboard)
                .delete(delete::delete_dashboard),
        )
}

use rubix_core::Dashboard;

use crate::auth::RequestPrincipal;
use crate::error::ApiError;
use crate::store::Store;

/// Authorize a read of `dashboard`: a site-scoped board checks the owning
/// site's org/slug; an org overview checks the org scope. Returns whether the
/// caller may see it (used both to gate a single get and to filter a list).
pub(crate) fn may_read_dashboard(
    principal: &RequestPrincipal,
    store: &Store,
    dashboard: &Dashboard,
) -> bool {
    match dashboard.site_id {
        Some(site_id) => match store.get_site(site_id) {
            Ok(site) => principal.authorize_site_read(&site.org, &site.slug).is_ok(),
            Err(_) => false,
        },
        None => principal
            .authorize_read(&crate::auth::Scope::org(&dashboard.org))
            .is_ok(),
    }
}

/// Authorize a write of `dashboard` (create/patch/delete), same scoping as
/// [`may_read_dashboard`] but requiring a write-capable role.
pub(crate) fn authorize_dashboard_write(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<uuid::Uuid>,
) -> Result<(), ApiError> {
    match site_id {
        Some(site_id) => {
            let site = store.get_site(site_id)?;
            principal.authorize_site_write(&site.org, &site.slug)?;
        }
        None => principal.authorize_write(&crate::auth::Scope::org(org))?,
    }
    Ok(())
}
