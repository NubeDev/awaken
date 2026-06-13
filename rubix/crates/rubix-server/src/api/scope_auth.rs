//! Shared org+site scope authorization for the entities that follow the uniform
//! tenancy model — dashboards, flows (boards), and rules. A scope is an `org`
//! plus an optional `site_id` (null = org-level, applying across the org):
//!
//! - **site-scoped** (`site_id = Some`): gate on the owning site's `org/slug`
//!   via [`RequestPrincipal::authorize_site_read`]/`authorize_site_write`.
//! - **org-level** (`site_id = None`): gate on the org scope directly.
//!
//! Reads a caller may not see are filtered before the wire; writes require a
//! write-capable role whose scope covers the target.

use uuid::Uuid;

use crate::auth::{RequestPrincipal, Scope};
use crate::error::ApiError;
use crate::store::Store;

/// Whether the principal may read a resource at `(org, site_id)`. Used both to
/// gate a single get and to filter a list pre-wire.
pub(crate) fn may_read_scope(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> bool {
    match site_id {
        Some(site_id) => match store.get_site(site_id) {
            Ok(site) => principal.authorize_site_read(&site.org, &site.slug).is_ok(),
            Err(_) => false,
        },
        None => principal.authorize_read(&Scope::org(org)).is_ok(),
    }
}

/// Authorize a write at `(org, site_id)` — create/patch/delete. Requires a
/// write-capable role whose scope covers the target.
pub(crate) fn authorize_scope_write(
    principal: &RequestPrincipal,
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> Result<(), ApiError> {
    match site_id {
        Some(site_id) => {
            let site = store.get_site(site_id)?;
            principal.authorize_site_write(&site.org, &site.slug)?;
        }
        None => principal.authorize_write(&Scope::org(org))?,
    }
    Ok(())
}
