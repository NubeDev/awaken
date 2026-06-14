//! awaken agent tools wired to server state: the BMS tool set and the
//! query-engine access backing the `query` tool.

mod board_access;
mod datasource_access;
mod query_access;
mod registry;
mod widget_access;

pub use board_access::StoreBoardAccess;
pub use datasource_access::RegistryDatasourceAccess;
pub use query_access::EngineQueryAccess;
pub use registry::{build_tools, build_tools_scoped};
pub use widget_access::StoreWidgetAccess;
