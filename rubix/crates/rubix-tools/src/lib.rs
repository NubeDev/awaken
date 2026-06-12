//! Rubix awaken AI tools.
//!
//! BMS tools an embedded awaken agent can call: read/write points (writes
//! gated through the priority array), SQL query, and board invocation. Each is
//! a [`awaken_runtime_contract::contract::tool::TypedTool`] over the
//! [`rubix_flow::PointAccess`] port, so tools stay free of the HTTP/store
//! layers and the host wires in a real implementation.

mod port;
mod prelude;
mod tool;

pub use port::{BoardAccess, PointAccess, QueryAccess, WidgetAccess};
pub use tool::{
    PinWidgetArgs, PinWidgetTool, QueryArgs, QueryTool, ReadPointArgs, ReadPointTool, RunBoardArgs,
    RunBoardTool, WritePointArgs, WritePointTool,
};
