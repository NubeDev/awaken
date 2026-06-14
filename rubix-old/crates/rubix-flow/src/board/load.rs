//! Load a [`BoardGraph`] into a runnable reflow `Network`: register the rubix
//! components the board uses, add each node with its config as metadata, and
//! wire the connections.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_network::connector::{ConnectionPoint, Connector};
use reflow_network::network::{Network, NetworkConfig};
use reflow_network::tracing::TracingConfig;

use super::registry::make_actor;
use super::schema::BoardGraph;
use crate::error::FlowError;
use crate::port::PointAccess;

impl BoardGraph {
    /// Build a `Network` for this board backed by `access`. Registers only the
    /// components the board references; an unknown component fails closed.
    pub fn load(&self, access: Arc<dyn PointAccess>) -> Result<Network, FlowError> {
        // reflow's default `NetworkConfig` enables a tracing client that dials a
        // `ws://localhost:8080` tracing server on every `Network::start`. No such
        // server runs here, so the default produces a reconnect storm (one dial
        // per board build, retried with backoff). Disable it explicitly — the
        // board runtime carries its own `tracing` instrumentation.
        let config = NetworkConfig {
            tracing: TracingConfig {
                enabled: false,
                ..TracingConfig::default()
            },
            ..NetworkConfig::default()
        };
        let mut network = Network::new(config);

        let mut registered = std::collections::HashSet::new();
        for node in &self.nodes {
            if registered.insert(node.component.clone()) {
                let actor = make_actor(&node.component, access.clone())
                    .ok_or_else(|| FlowError::UnknownComponent(node.component.clone()))?;
                network
                    .register_actor_arc(&node.component, actor)
                    .map_err(|e| FlowError::Build(e.to_string()))?;
            }
        }

        for node in &self.nodes {
            let metadata: HashMap<String, serde_json::Value> = node.config.clone();
            network
                .add_node(&node.id, &node.component, Some(metadata))
                .map_err(|e| FlowError::Build(e.to_string()))?;
        }

        for c in &self.connections {
            network.add_connection(Connector::new(
                ConnectionPoint::new(&c.from_node, &c.from_port, None),
                ConnectionPoint::new(&c.to_node, &c.to_port, None),
            ));
        }

        Ok(network)
    }
}
