//! Stored-rule routes — wiring only. Org-scoped CRUD plus the referencing
//! (change-impact) listing for the rules engine's composition.

pub(crate) mod create;
pub(crate) mod delete;
pub(crate) mod dry_run;
pub(crate) mod dto;
pub(crate) mod get;
pub(crate) mod list;
pub(crate) mod referencing;
pub(crate) mod update;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/orgs/{org}/rules",
            post(create::create_rule).get(list::list_rules),
        )
        // Static segment registered before `{name}` so the dry-run path is never
        // captured as a rule named "dry-run".
        .route(
            "/api/v1/orgs/{org}/rules/dry-run",
            post(dry_run::dry_run_rule),
        )
        .route(
            "/api/v1/orgs/{org}/rules/{name}",
            get(get::get_rule)
                .put(update::update_rule)
                .delete(delete::delete_rule),
        )
        .route(
            "/api/v1/orgs/{org}/rules/{name}/referencing",
            get(referencing::referencing_rules),
        )
}
