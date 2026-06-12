//! Rubix BMS backend server.
//!
//! Axum HTTP API (OpenAPI 3.1 via utoipa) over a SQLite store. This is the
//! supervisory backend from STACK-DEISGN.md; zenoh transport, reflow engine,
//! and DataFusion query layers attach to the same store later.

pub mod agent;
pub mod api;
pub mod bus;
pub mod dispatch;
pub mod error;
pub mod flow;
pub mod his;
pub mod scheduler;
pub mod store;
pub mod supervisor;
pub mod tools;

use std::sync::Arc;

use axum::Router;
use tower_http::trace::TraceLayer;

use awaken_runtime::AgentRuntime;
use rubix_query::{HisTier, QueryEngine};

use crate::bus::ZenohBus;
use crate::store::Store;

#[derive(Clone)]
pub struct AppState {
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
    /// Embedded awaken agent runtime over the BMS tools. `None` unless
    /// `RUBIX_AI=1`; the `/agent/chat` route returns 503 when absent.
    pub agent: Option<Arc<AgentRuntime>>,
    /// Priority level AI/agent writes are clamped to (1..=16); writes from
    /// agents may not command above (numerically below) this level outright —
    /// they escalate for human approval instead.
    pub ai_min_priority: u8,
    /// Lowest priority slot an agent write may reach *with* human approval
    /// (1..=`ai_min_priority`). Slots below this are operator-reserved and
    /// hard-refused. Defaults to 1 (escalate the whole band above the ceiling).
    pub ai_escalation_floor: u8,
}

pub fn app(state: AppState) -> Router {
    api::router(state).layer(TraceLayer::new_for_http())
}
