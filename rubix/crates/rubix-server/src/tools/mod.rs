//! awaken agent tools wired to server state: the BMS tool set and the
//! query-engine access backing the `query` tool.

mod query_access;
mod registry;

pub use query_access::EngineQueryAccess;
pub use registry::build_tools;
