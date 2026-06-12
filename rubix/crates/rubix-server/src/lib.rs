//! Rubix BMS backend server.
//!
//! Axum HTTP API (OpenAPI 3.1 via utoipa) over a SQLite store. This is the
//! supervisory backend from STACK-DEISGN.md; zenoh transport, reflow engine,
//! and DataFusion query layers attach to the same store later.

#[cfg(not(any(feature = "edge", feature = "cloud")))]
compile_error!(
    "rubix-server needs at least one deployment profile feature: \
     build with --features edge (default) and/or --features cloud"
);

pub mod agent;
pub mod api;
pub mod auth;
pub mod bus;
pub mod dispatch;
pub mod error;
pub mod flow;
pub mod his;
pub mod profile;
pub mod scheduler;
pub mod store;
pub mod supervisor;
pub mod tools;

use std::sync::Arc;

use axum::Router;
use tower_http::trace::TraceLayer;

use awaken_runtime::AgentRuntime;
use rubix_query::{HisTier, QueryEngine};

use crate::auth::Authenticator;
use crate::bus::ZenohBus;
use crate::profile::Profile;
use crate::store::Store;

#[derive(Clone)]
pub struct AppState {
    /// The deployment profile this node runs as (edge/cloud), resolved once at
    /// boot. Carries the per-profile defaults later layers attach to.
    pub profile: Profile,
    pub store: Store,
    /// Zenoh data plane. `None` when the server runs without transport (tests,
    /// HTTP-only mode); handlers publish `cur` through it when present.
    pub bus: Option<ZenohBus>,
    /// DataFusion SQL surface over the store. `None` in HTTP-only/test modes;
    /// the `/query` route returns 503 when absent.
    pub query: Option<QueryEngine>,
    /// Parquet `his` cold tier (`object_store`). `None` keeps `his` SQLite-only;
    /// when present, `his` queries union both tiers and `/his/flush` ages rows
    /// out of SQLite into Parquet partitions.
    pub his_tier: Option<HisTier>,
    /// Embedded awaken agent runtime over the BMS tools, unscoped (every site
    /// reachable). `None` unless `RUBIX_AI=1`; the `/agent/chat` route returns
    /// 503 when absent. Used directly for runs that carry no tenant scope.
    pub agent: Option<Arc<AgentRuntime>>,
    /// The inputs to rebuild a runtime whose tools are confined to a tenant
    /// `{org}/{site}`. `Some` whenever `agent` is, so a chat run for a scoped
    /// principal or a dispatch run for a scoped spark gets a tenant-confined
    /// tool set. See [`agent::build_scoped_runtime`].
    pub agent_blueprint: Option<agent::RuntimeBlueprint>,
    /// Priority level AI/agent writes are clamped to (1..=16); writes from
    /// agents may not command above (numerically below) this level outright —
    /// they escalate for human approval instead.
    pub ai_min_priority: u8,
    /// Lowest priority slot an agent write may reach *with* human approval
    /// (1..=`ai_min_priority`). Slots below this are operator-reserved and
    /// hard-refused. Defaults to 1 (escalate the whole band above the ceiling).
    pub ai_escalation_floor: u8,
    /// Bearer-token verifier. `Some` only when auth is enforced (cloud profile
    /// with an OIDC issuer configured); the auth middleware is installed and the
    /// `tokens` admin routes accept requests. `None` on edge — requests pass
    /// without a principal and RBAC gates are no-ops (today's behavior).
    pub authenticator: Option<Authenticator>,
}

pub fn app(state: AppState) -> Router {
    let authenticator = state.authenticator.clone();
    let router = api::router(state);
    // Install the enforcing layer only when auth is configured. On edge there is
    // no authenticator, so no middleware is added and requests pass untouched.
    let router = match authenticator {
        Some(auth) => router.layer(axum::middleware::from_fn_with_state(
            auth,
            auth::middleware::require_auth,
        )),
        None => router,
    };
    router.layer(TraceLayer::new_for_http())
}
