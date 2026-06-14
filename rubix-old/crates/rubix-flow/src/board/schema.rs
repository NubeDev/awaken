//! Board wire format: the JSON a reflow control/rule board is stored as, in
//! SQLite/Postgres and deployed over zenoh. Kept independent of reflow's
//! internal graph types so the stored format stays stable across engine bumps.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A board: a set of nodes wired by connections. Loaded into a reflow
/// `Network` by [`super::load`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardGraph {
    pub nodes: Vec<BoardNode>,
    #[serde(default)]
    pub connections: Vec<BoardConnection>,
}

/// One node: a unique `id`, the `component` naming a registered actor
/// (`read_point`, `write_point`, `query_his`, …), and a `config` blob that
/// becomes the actor's `ActorConfig` (e.g. `point`, `priority`, `limit`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardNode {
    pub id: String,
    pub component: String,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

/// A directed edge from one node's outport to another's inport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardConnection {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}
