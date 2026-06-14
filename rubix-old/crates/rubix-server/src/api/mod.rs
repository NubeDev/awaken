//! HTTP API. One file per route verb; this module is router wiring only.

mod agent;
mod audit;
mod blocking;
mod boards;
mod command;
mod dashboards;
mod datasources;
mod equips;
mod grants;
mod health;
mod his;
mod openapi;
mod orgs;
mod points;
mod preferences;
mod query;
mod rules;
mod runs;
mod scope_auth;
mod sites;
mod nav;
mod sparks;
mod tag_query;
mod tags;
mod teams;
mod time_range;
mod units_ctx;
mod tokens;
mod users;
mod whoami;
mod widgets;

pub use openapi::ApiDoc;
pub use units_ctx::{UnitsCtx, UnitsMode};

use axum::routing::get;
use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/api-docs/openapi.json", get(openapi::openapi_json))
        .route("/api/v1/whoami", get(whoami::whoami))
        .merge(sites::router())
        .merge(orgs::router())
        .merge(equips::router())
        .merge(points::router())
        .merge(command::router())
        .merge(his::router())
        .merge(sparks::router())
        .merge(widgets::router())
        .merge(dashboards::router())
        // The query + datasource surfaces carry unit-bearing series, so they get
        // the Accept-Units layer: it resolves the caller's prefs once and stashes
        // a `UnitsCtx` for the handlers to convert with. Other routes don't emit
        // converted quantities, so they skip the per-request prefs round-trip.
        .merge(
            query::router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                units_ctx::accept_units,
            )),
        )
        .merge(preferences::router())
        .merge(
            datasources::router().layer(axum::middleware::from_fn_with_state(
                state.clone(),
                units_ctx::accept_units,
            )),
        )
        .merge(rules::router())
        .merge(boards::router())
        .merge(agent::router())
        .merge(runs::router())
        .merge(tokens::router())
        .merge(users::router())
        .merge(teams::router())
        .merge(grants::router())
        .merge(tags::router())
        .merge(nav::router())
        .merge(audit::router())
        .merge(crate::mcp::router())
        .with_state(state)
}
