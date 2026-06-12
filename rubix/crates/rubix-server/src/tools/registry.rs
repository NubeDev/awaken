//! Build the BMS agent tool set from server state. The host hands these to an
//! embedded awaken runtime; point tools share the store, the query tool the
//! DataFusion engine.

use std::sync::Arc;

use awaken_runtime_contract::contract::tool::Tool;
use rubix_tools::{PinWidgetTool, QueryTool, ReadPointTool, RunBoardTool, WritePointTool};

use super::board_access::StoreBoardAccess;
use super::query_access::EngineQueryAccess;
use super::widget_access::StoreWidgetAccess;
use crate::flow::StorePointAccess;
use crate::AppState;

/// Construct the agent-callable BMS tools. `read_point`/`write_point`/
/// `run_board` are always available (store-backed); `query` is included only
/// when the DataFusion engine is configured.
pub fn build_tools(state: &AppState) -> Vec<Arc<dyn Tool>> {
    let access = Arc::new(StorePointAccess::new(state.store.clone()));
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ReadPointTool::new(access.clone())),
        Arc::new(WritePointTool::with_escalation_floor(
            access,
            state.ai_min_priority,
            state.ai_escalation_floor,
        )),
        Arc::new(RunBoardTool::new(Arc::new(StoreBoardAccess::new(
            state.store.clone(),
        )))),
        Arc::new(PinWidgetTool::new(Arc::new(StoreWidgetAccess::new(
            state.store.clone(),
        )))),
    ];
    if let Some(engine) = &state.query {
        let query_access = Arc::new(EngineQueryAccess::new(engine.clone()));
        tools.push(Arc::new(QueryTool::new(query_access)));
    }
    tools
}
