//! Admin & management routes — principals, grants, tenants, devices.
//!
//! The control-plane surface (`rubix/docs/design/ADMIN-API.md`): full CRUD over
//! identities and their capability grants, cloud tenant onboarding, and the edge
//! device registry. Every mutation is admin-guarded at the transport (or
//! root/system for tenant onboarding) and audited at the gate. One file per
//! resource; this barrel merges them into a router. The tenant routes are always
//! mounted (one binary edge-to-cloud) and branch on the profile at runtime.

mod devices;
mod grants;
pub(crate) mod guard;
mod principals;
mod tenants;

use axum::Router;
use axum::routing::{get, post, put};

use crate::state::AppState;

use devices::{
    create_device_route, delete_device_route, get_device_route, list_devices_route,
    update_device_route,
};
use grants::{delete_grant_route, list_grants_route, put_grant_route};
use principals::{
    create_principal_route, delete_principal_route, get_principal_route, list_principals_route,
    update_principal_route,
};
use tenants::{create_tenant_route, delete_tenant_route, list_tenants_route};

/// The admin routes mounted under `/principals`, `/tenants`, and `/devices`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/principals",
            post(create_principal_route).get(list_principals_route),
        )
        .route(
            "/principals/:subject",
            get(get_principal_route)
                .patch(update_principal_route)
                .delete(delete_principal_route),
        )
        .route("/principals/:subject/grants", get(list_grants_route))
        .route(
            "/principals/:subject/grants/:capability",
            put(put_grant_route).delete(delete_grant_route),
        )
        .route(
            "/tenants",
            post(create_tenant_route).get(list_tenants_route),
        )
        .route("/tenants/:id", axum::routing::delete(delete_tenant_route))
        .route(
            "/devices",
            post(create_device_route).get(list_devices_route),
        )
        .route(
            "/devices/:id",
            get(get_device_route)
                .patch(update_device_route)
                .delete(delete_device_route),
        )
}
