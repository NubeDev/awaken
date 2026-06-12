//! Rubix BMS backend server.
//!
//! Axum HTTP API (OpenAPI 3.1 via utoipa) over a SQLite store. This is the
//! supervisory backend from STACK-DEISGN.md; zenoh transport, reflow engine,
//! and DataFusion query layers attach to the same store later.

pub mod agent;
pub mod api;
pub mod bus;
pub mod error;
pub mod flow;
pub mod store;
pub mod supervisor;
pub mod tools;

use std::sync::Arc;

use axum::Router;
use tower_http::trace::TraceLayer;

use awaken_runtime::AgentRuntime;
use rubix_query::QueryEngine;

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
    /// Embedded awaken agent runtime over the BMS tools. `None` unless
    /// `RUBIX_AI=1`; the `/agent/chat` route returns 503 when absent.
    pub agent: Option<Arc<AgentRuntime>>,
    /// Priority level AI/agent writes are clamped to (1..=16); writes from
    /// agents may not command above (numerically below) this level.
    pub ai_min_priority: u8,
}

pub fn app(state: AppState) -> Router {
    api::router(state).layer(TraceLayer::new_for_http())
}
