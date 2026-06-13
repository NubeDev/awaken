//! Evaluate one stored board against the store. This is the single place a
//! scheduled or subscription-triggered board actually runs, so the interval
//! and subscription loops share identical semantics: load the board's graph,
//! run it once over [`StorePointAccess`] (writes go through the priority
//! array), and log the outcome. A scheduled board's durable effect is its
//! writes and emitted sparks; its transient node outputs are captured into the
//! in-memory [`BoardOutputs`] cache so a client can see what an enabled board
//! is producing (latest-only, not history).

use std::sync::Arc;

use awaken_runtime::AgentRuntime;
use rubix_datasource::DatasourceRegistry;
use rubix_flow::BoardGraph;

use super::outputs::BoardOutputs;
use crate::bus::ZenohBus;
use crate::flow::StorePointAccess;
use crate::store::Store;

/// The backend services a scheduled board run binds to its [`StorePointAccess`]:
/// the store (point read/command, history), the bus (`emit_spark` publishing and
/// subscription triggers), the agent (`agent_call`), and the datasource registry
/// (`datasource` nodes). Bundled so the loops and `evaluate` thread one value
/// rather than a widening parameter list. Cheaply cloned (each field is already
/// `Clone`/`Arc`).
#[derive(Clone)]
pub(super) struct BoardRunDeps {
    pub store: Store,
    pub bus: Option<ZenohBus>,
    pub agent: Option<Arc<AgentRuntime>>,
    pub datasources: Option<Arc<DatasourceRegistry>>,
    pub outputs: BoardOutputs,
}

/// Run `graph` once over the store. Logs at debug on success and warn on
/// failure; never panics, so a single bad board cannot take down the loop. On
/// success the run's outputs replace this board's latest entry in the cache.
pub(super) async fn evaluate(slug: &str, graph: &BoardGraph, deps: &BoardRunDeps) {
    let access = Arc::new(
        StorePointAccess::with_bus(deps.store.clone(), deps.bus.clone())
            .with_agent(deps.agent.clone())
            .with_org(graph.tenant_org())
            .with_datasources(deps.datasources.clone()),
    );
    match graph.run(access).await {
        Ok(outputs) => {
            deps.outputs
                .record(slug, &outputs, chrono::Utc::now().to_rfc3339());
            tracing::debug!(board = slug, outputs = outputs.len(), "scheduled board ran");
        }
        Err(e) => {
            tracing::warn!(board = slug, error = %e, "scheduled board failed");
        }
    }
}
