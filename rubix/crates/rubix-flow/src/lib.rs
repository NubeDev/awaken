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

pub use board::{BoardConnection, BoardGraph, BoardNode, NodeOutput, COMPONENTS};
pub use error::FlowError;
pub use node::{QueryHisActor, ReadPointActor, WritePointActor};
pub use port::PointAccess;
