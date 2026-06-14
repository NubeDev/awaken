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
use uuid::Uuid;

use crate::api::scope_auth::{authorize_resource_write, may_read_resource, resource_ref};
use crate::auth::RequestPrincipal;
use crate::error::ApiError;
use crate::store::Store;

/// The Layer-2 grant address of an existing dashboard (`dashboard:<id>`).
fn dashboard_ref(id: Uuid) -> String {
    resource_ref("dashboard", &id.to_string())
}

/// Authorize a read of `dashboard` via the two-layer check: Layer-1 scope-role
/// (site-scoped → owning site's org/slug; org overview → org scope) OR a Layer-2
/// read grant on this dashboard. Used both to gate a single get and to filter a
/// list (a granted member sees it even without scope read).
pub(crate) fn may_read_dashboard(
    principal: &RequestPrincipal,
    store: &Store,
    dashboard: &Dashboard,
) -> bool {
    may_read_resource(
        principal,
        store,
        &dashboard.org,
        dashboard.site_id,
        "dashboard",
        &dashboard_ref(dashboard.id),
    )
}

/// Authorize a write of a dashboard (patch/delete of an existing one) via the
/// two-layer check. `id` addresses the dashboard for the Layer-2 grant lookup.
pub(crate) fn authorize_dashboard_write_existing(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    id: Uuid,
) -> Result<(), ApiError> {
    authorize_resource_write(principal, store, org, site_id, "dashboard", &dashboard_ref(id))
}

/// Authorize creation of a new dashboard. There is no resource id to grant
/// against yet, so this is Layer-1 only (a `dashboard:*` wildcard grant within
/// the scope also authorizes it via the two-layer path).
pub(crate) fn authorize_dashboard_write(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> Result<(), ApiError> {
    // `*` lets a "writes all dashboards in this org" wildcard grant create, too.
    authorize_resource_write(principal, store, org, site_id, "dashboard", "*")
}
