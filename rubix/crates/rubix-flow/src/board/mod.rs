//! Boards: the stored JSON graph format and its loader into a reflow Network.

mod load;
mod registry;
mod schema;

pub use registry::COMPONENTS;
pub use schema::{BoardConnection, BoardGraph, BoardNode};
