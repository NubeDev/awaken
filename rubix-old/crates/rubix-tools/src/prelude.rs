//! Shared imports for tool implementations: the awaken `TypedTool` contract
//! and the rubix BMS access port.

pub use async_trait::async_trait;
pub use awaken_runtime_contract::contract::tool::{
    ToolCallContext, ToolError, ToolOutput, ToolResult, TypedTool,
};
pub use schemars::JsonSchema;
pub use serde::Deserialize;
pub use std::sync::Arc;

pub use rubix_flow::PointAccess;
