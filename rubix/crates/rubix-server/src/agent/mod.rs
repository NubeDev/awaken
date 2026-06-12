//! Embedded awaken agent: builds an [`awaken_runtime::AgentRuntime`] over the
//! BMS tool set and exposes a chat turn. The agent reads/commands points and
//! queries history through the same gated tools the HTTP API uses.

mod capture_sink;
mod persist;
mod run_record;
mod runtime;
mod scoped;

pub use persist::run_and_persist;
pub use run_record::{PendingWrite, RunOrigin, RunRecord, RunStatus};
pub use runtime::{
    build_runtime, build_runtime_with_executor, build_scoped_runtime, RuntimeBlueprint, AGENT_ID,
};
pub use scoped::runtime_for_scope;
