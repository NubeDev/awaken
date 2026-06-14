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

/// Quiet interval between output polls. After a poll yields no new packets the
/// graph is considered settled. Control boards are shallow and low-rate; a few
/// graph hops settle within one interval.
const SETTLE: Duration = Duration::from_millis(50);

/// Upper bound on total settle time. A plain control board settles in one
/// [`SETTLE`] interval; an awaited `agent_call` node blocks its actor task on an
/// LLM run, so the budget must cover that round-trip before its downstream
/// nodes fire. Bounded so a hung run cannot wedge a single-shot board run.
const MAX_SETTLE: Duration = Duration::from_secs(120);

/// Link/value quality, the status flag every Niagara/Sedona link carries — so a
/// retained value is self-describing rather than "a value that might be stale or
/// a fault". Derived from the port and value at capture time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    /// A normal value on a normal port.
    Ok,
    /// A value emitted on a node's `error` port — the node failed this run.
    Fault,
    /// The port carried a null/absent value.
    Null,
}

impl Quality {
    /// Classify a captured `(port, value)` pair. An `error` port is a fault; a
    /// JSON null is null; everything else is ok.
    pub fn of(port: &str, value: &serde_json::Value) -> Self {
        if port == "error" {
            Quality::Fault
        } else if value.is_null() {
            Quality::Null
        } else {
            Quality::Ok
        }
    }

    /// The lowercase wire token (`ok`/`fault`/`null`).
    pub fn as_str(self) -> &'static str {
        match self {
            Quality::Ok => "ok",
            Quality::Fault => "fault",
            Quality::Null => "null",
        }
    }
}

/// One node's collected outport output after a run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeOutput {
    pub node: String,
    pub port: String,
    /// JSON projection of the reflow message emitted on `port`.
    pub value: serde_json::Value,
    /// Quality of this value, derived from the port and value.
    pub quality: Quality,
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

        // Drain outputs until a poll interval yields nothing new (the graph has
        // settled) or the max budget is hit. `read_actor_output` consumes each
        // node's outport channel, so packets are accumulated across polls — a
        // blocking node that emits late (an awaited `agent_call`) is still caught.
        let mut outputs = Vec::new();
        let deadline = tokio::time::Instant::now() + MAX_SETTLE;
        loop {
            tokio::time::sleep(SETTLE).await;
            let mut drained = false;
            for node in &self.nodes {
                for (port, msg) in network.read_actor_output(&node.id) {
                    drained = true;
                    let value = serde_json::Value::from(msg);
                    let quality = Quality::of(&port, &value);
                    outputs.push(NodeOutput {
                        node: node.id.clone(),
                        port,
                        value,
                        quality,
                    });
                }
            }
            if !drained || tokio::time::Instant::now() >= deadline {
                break;
            }
        }
        network.shutdown();
        Ok(outputs)
    }

    /// Ids of nodes with no inbound connection — the board's entry points.
    pub(crate) fn source_nodes(&self) -> Vec<String> {
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
