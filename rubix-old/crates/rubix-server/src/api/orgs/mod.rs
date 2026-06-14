//! Org/tenant routes — a derived management surface over `Site` rows.
//!
//! There is no `orgs` table: an org is the `org` string carried by its sites
//! (design increment **B1** in `docs/design/crud-and-tenancy.md`). These routes
//! give the admin UI a first-class "tenant list" (GET, grouped from the sites
//! the principal may see) and a one-call provision action (POST, which creates
//! the first site under a new org). No new isolation boundary — every read is
//! filtered through `authorize_site_read` exactly like `list_sites`.

pub(crate) mod create;
pub(crate) mod list;

use axum::routing::get;
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new().route(
        "/api/v1/orgs",
        get(list::list_orgs).post(create::provision_org),
    )
}
