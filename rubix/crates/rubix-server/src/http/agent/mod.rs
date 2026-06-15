//! Agent resource routes — provision, list, memory, and ask.
//!
//! The transport for the agent runtime (`rubix/docs/design/AGENT.md`). Provisioning
//! and listing are admin actions over agent-as-extension principals; the memory
//! routes are the two enforcement points (recall on the scoped session, persist
//! through the gate); ask runs the brain. One file per concern; this barrel merges
//! them into a router mounted at the crate root.

mod ask;
mod list;
mod memory;
mod provision;

use axum::Router;
use axum::routing::post;

use crate::state::AppState;

use ask::ask_agent_route;
use list::list_agents_route;
use memory::{persist_memory_route, recall_memory_route};
use provision::provision_agent_route;

/// The agent routes mounted under `/agent`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agent", post(provision_agent_route).get(list_agents_route))
        .route("/agent/memory/recall", post(recall_memory_route))
        .route("/agent/memory/persist", post(persist_memory_route))
        .route("/agent/ask", post(ask_agent_route))
}
