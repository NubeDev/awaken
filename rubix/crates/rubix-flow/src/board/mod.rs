//! Boards: the stored JSON graph format and its loader into a reflow Network.

mod component_schema;
mod engine;
mod load;
mod registry;
mod run;
mod schema;
mod tenant;

pub use component_schema::{
    component_schemas, ComponentKind, ComponentSchema, ConfigField, FieldType, PortSchema, PortType,
};
pub use engine::BoardEngine;
pub use registry::COMPONENTS;
pub use run::NodeOutput;
pub use schema::{BoardConnection, BoardGraph, BoardNode};
