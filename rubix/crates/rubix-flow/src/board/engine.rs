//! A persistent board runtime: one started reflow `Network` kept alive across
//! many scans, instead of the build→run→shutdown-per-tick of [`BoardGraph::run`].
//!
//! [`BoardEngine`] is the Niagara/Sedona-style scan engine. The host spawns one
//! per enabled interval board and ticks it on a cadence; the actor network — and
//! each actor's state — survives between scans, so the `[INPORT CLOSED]` churn of
//! the rebuild model is gone and stateful nodes no longer need a process-global
//! workaround.
//!
//! ## Observing values
//!
//! reflow consumes a *connected* node's outport with an internal fan-out
//! forwarder, so `read_actor_output` reliably yields only terminal (no-outbound)
//! nodes. Every wired link, however, emits a `NetworkEvent::MessageSent` on the
//! network event stream. The engine therefore reads interior link values from
//! that event stream and terminal values from `read_actor_output`, together
//! giving a complete per-(node, port) snapshot. Draining the (unbounded) event
//! stream every scan also keeps it from growing without bound.
//!
//! ## Teardown
//!
//! Dropping the engine drops its `Network`, which closes every actor inport and
//! outport channel; the per-actor tasks and the fan-out forwarder tasks then
//! unwind on their own. The host drops (never merely "stops") the engine on
//! disable or republish, so no forwarder task is left running.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use reflow_actor::message::Message;
use reflow_network::network::{Network, NetworkEvent};

use super::run::NodeOutput;
use super::schema::BoardGraph;
use crate::error::FlowError;
use crate::port::PointAccess;

/// How long a scan lets a tick propagate before draining. Interior hops settle
/// in well under this; a node that emits later (an awaited `agent_call`) is
/// captured on a subsequent scan — the retained-value model is eventually
/// consistent rather than blocking the scan on the slowest node.
const SCAN_SETTLE: Duration = Duration::from_millis(50);

/// A started board network kept alive across scans. Owns the sole [`Network`]
/// handle; dropping the engine tears the network down (see module docs).
pub struct BoardEngine {
    network: Network,
    /// Source nodes (no inbound connection) — re-ticked every scan.
    sources: Vec<String>,
    /// Terminal nodes (no outbound connection) — drained directly each scan,
    /// since no fan-out forwarder competes for their outport.
    terminals: Vec<String>,
    /// Network event stream; the source of interior link values.
    events: flume::Receiver<NetworkEvent>,
    /// Latest value per `(node, port)`, retained across scans.
    values: HashMap<(String, String), NodeOutput>,
}

impl BoardGraph {
    /// Build and start a persistent [`BoardEngine`] for this board over `access`.
    /// Requires a Tokio runtime (the network spawns per-actor tasks). The engine
    /// runs nothing until [`BoardEngine::scan`] is called.
    pub fn spawn_engine(&self, access: Arc<dyn PointAccess>) -> Result<BoardEngine, FlowError> {
        let mut network = self.load(access)?;
        network
            .start()
            .map_err(|e| FlowError::Build(format!("network start: {e}")))?;
        let events = network.get_event_receiver();
        Ok(BoardEngine {
            network,
            sources: self.source_nodes(),
            terminals: self.terminal_nodes(),
            events,
            values: HashMap::new(),
        })
    }

    /// Ids of nodes with no outbound connection — the board's sinks. Their
    /// outport is not consumed by a forwarder, so it is drained directly.
    fn terminal_nodes(&self) -> Vec<String> {
        let sources: HashSet<&str> = self
            .connections
            .iter()
            .map(|c| c.from_node.as_str())
            .collect();
        self.nodes
            .iter()
            .filter(|n| !sources.contains(n.id.as_str()))
            .map(|n| n.id.clone())
            .collect()
    }
}

impl BoardEngine {
    /// Run one scan: re-tick every source node, let the network propagate, then
    /// fold this scan's link and terminal values into the retained snapshot.
    pub async fn scan(&mut self) {
        for id in &self.sources {
            // The injected value is irrelevant — read/query nodes pull from
            // config; what matters is that the actor's behavior runs this scan.
            let _ = self.network.send_to_actor(id, "trigger", Message::Flow);
        }
        tokio::time::sleep(SCAN_SETTLE).await;
        self.drain();
    }

    /// Fold pending events (interior links) and terminal outputs into `values`.
    fn drain(&mut self) {
        // Interior links: every wired edge emits a MessageSent. Drains the whole
        // event backlog (non-MessageSent events are discarded), so the unbounded
        // event channel never grows across scans.
        while let Ok(event) = self.events.try_recv() {
            if let NetworkEvent::MessageSent {
                from_actor,
                from_port,
                message,
                ..
            } = event
            {
                let value = serde_json::Value::from(message);
                self.values.insert(
                    (from_actor.clone(), from_port.clone()),
                    NodeOutput {
                        node: from_actor,
                        port: from_port,
                        value,
                    },
                );
            }
        }
        // Terminal nodes: no forwarder competes, so read their outport directly.
        for id in &self.terminals {
            for (port, msg) in self.network.read_actor_output(id) {
                self.values.insert(
                    (id.clone(), port.clone()),
                    NodeOutput {
                        node: id.clone(),
                        port,
                        value: serde_json::Value::from(msg),
                    },
                );
            }
        }
    }

    /// The current retained value of every observed `(node, port)` link.
    pub fn current_values(&self) -> Vec<NodeOutput> {
        self.values.values().cloned().collect()
    }
}
