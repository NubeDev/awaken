//! Rubix reflow actors.
//!
//! Custom nodes that bridge reflow control/rule boards to the BMS: point
//! read/write (always through the priority array), and history query. Nodes
//! depend on the [`PointAccess`] port, not on the server — `rubix-server`
//! implements the port and runs the boards.
//!
//! A [`BoardGraph`] is the stored JSON format; [`BoardGraph::load`] builds a
//! runnable reflow `Network` from it.

mod board;
mod error;
mod node;
mod port;
mod state;

pub use board::{
    component_schemas, BoardConnection, BoardEngine, BoardGraph, BoardNode, ComponentKind,
    ComponentSchema, ConfigField, FieldType, NodeOutput, PortSchema, PortType, Quality, COMPONENTS,
};
pub use error::{FlowAccessError, FlowError};
pub use node::{
    frame_from_his, map_severity, AgentCallActor, EmitSparkActor, QueryHisActor, ReadPointActor,
    RuleActor, TriggerActor, WritePointActor,
};
pub use port::{
    AgentOutcome, AgentRequest, DatasourceQuery, PointAccess, SparkDraft, WatchSample,
};
pub use state::{NodeState, StatePolicy};
