//! Boards: the stored JSON graph format and its loader into a reflow Network.

mod load;
mod registry;
mod run;
mod schema;

pub use registry::COMPONENTS;
pub use run::NodeOutput;
pub use schema::{BoardConnection, BoardGraph, BoardNode};
