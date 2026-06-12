//! Execute a loaded board: start its `Network`, tick the source nodes, let the
//! actors propagate, and collect every node's outport packets. A single-shot
//! run — the caller drives one evaluation of the board and reads the result.
//! Scheduled/triggered control loops layer on top by calling [`BoardGraph::run`]
//! on a cadence or a zenoh subscription.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use reflow_actor::message::Message;

use super::schema::BoardGraph;
use crate::error::FlowError;
use crate::port::PointAccess;

/// How long to let actors propagate after the source tick before draining
/// outputs. Control boards are shallow and low-rate; a few graph hops settle
/// well within this window.
const SETTLE: Duration = Duration::from_millis(50);

/// One node's collected outport output after a run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeOutput {
    pub node: String,
    pub port: String,
    /// JSON projection of the reflow message emitted on `port`.
    pub value: serde_json::Value,
}

impl BoardGraph {
    /// Load and evaluate this board once against `access`, returning every
    /// outport packet produced. Source nodes (those with no inbound connection)
    /// are ticked on their first inport to kick the graph; downstream nodes fire
    /// as messages reach them. Requires a Tokio runtime (the network spawns
    /// per-actor tasks).
    pub async fn run(&self, access: Arc<dyn PointAccess>) -> Result<Vec<NodeOutput>, FlowError> {
        let mut network = self.load(access)?;
        network
            .start()
            .map_err(|e| FlowError::Build(format!("network start: {e}")))?;

        for node_id in self.source_nodes() {
            // Tick the source node on its first declared inport. The injected
            // value is irrelevant — read/query nodes pull from config, write
            // nodes that are sources take their value from config — what matters
            // is that the actor's behavior runs.
            let _ = network.send_to_actor(&node_id, "trigger", Message::Flow);
        }

        tokio::time::sleep(SETTLE).await;

        let mut outputs = Vec::new();
        for node in &self.nodes {
            for (port, msg) in network.read_actor_output(&node.id) {
                outputs.push(NodeOutput {
                    node: node.id.clone(),
                    port,
                    value: serde_json::Value::from(msg),
                });
            }
        }
        network.shutdown();
        Ok(outputs)
    }

    /// Ids of nodes with no inbound connection — the board's entry points.
    fn source_nodes(&self) -> Vec<String> {
        let sinks: HashSet<&str> = self
            .connections
            .iter()
            .map(|c| c.to_node.as_str())
            .collect();
        self.nodes
            .iter()
            .filter(|n| !sinks.contains(n.id.as_str()))
            .map(|n| n.id.clone())
            .collect()
    }
}
