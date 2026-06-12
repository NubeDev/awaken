//! Build the BMS agent tool set from server state. The host hands these to an
//! embedded awaken runtime; point tools share the store, the query tool the
//! DataFusion engine.

use std::sync::Arc;

use awaken_runtime_contract::contract::tool::Tool;
use rubix_tools::{QueryTool, ReadPointTool, WritePointTool};

use super::query_access::EngineQueryAccess;
use crate::flow::StorePointAccess;
use crate::AppState;

/// Construct the agent-callable BMS tools. `read_point`/`write_point` are
/// always available (store-backed); `query` is included only when the
/// DataFusion engine is configured.
pub fn build_tools(state: &AppState) -> Vec<Arc<dyn Tool>> {
    let access = Arc::new(StorePointAccess::new(state.store.clone()));
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ReadPointTool::new(access.clone())),
        Arc::new(WritePointTool::new(access, state.ai_min_priority)),
    ];
    if let Some(engine) = &state.query {
        let query_access = Arc::new(EngineQueryAccess::new(engine.clone()));
        tools.push(Arc::new(QueryTool::new(query_access)));
    }
    tools
}
