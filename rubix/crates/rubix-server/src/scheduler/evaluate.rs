//! Evaluate one stored board against the store. This is the single place a
//! scheduled or subscription-triggered board actually runs, so the interval
//! and subscription loops share identical semantics: load the board's graph,
//! run it once over [`StorePointAccess`] (writes go through the priority
//! array), and log the outcome. Outputs are not returned anywhere — a
//! scheduled board's effect is its writes and (later) its emitted sparks.

use std::sync::Arc;

use awaken_runtime::AgentRuntime;
use rubix_flow::BoardGraph;

use crate::bus::ZenohBus;
use crate::flow::StorePointAccess;
use crate::store::Store;

/// Run `graph` once over the store. Logs at debug on success and warn on
/// failure; never panics, so a single bad board cannot take down the loop.
/// The bus, when present, lets the board's `emit_spark` nodes publish findings;
/// the agent, when present, backs `agent_call` nodes.
pub(super) async fn evaluate(
    slug: &str,
    graph: &BoardGraph,
    store: &Store,
    bus: &Option<ZenohBus>,
    agent: &Option<Arc<AgentRuntime>>,
) {
    let access = Arc::new(
        StorePointAccess::with_bus(store.clone(), bus.clone())
            .with_agent(agent.clone())
            .with_org(graph.tenant_org()),
    );
    match graph.run(access).await {
        Ok(outputs) => {
            tracing::debug!(board = slug, outputs = outputs.len(), "scheduled board ran");
        }
        Err(e) => {
            tracing::warn!(board = slug, error = %e, "scheduled board failed");
        }
    }
}
