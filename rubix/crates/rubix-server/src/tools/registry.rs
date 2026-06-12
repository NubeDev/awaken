//! Build the BMS agent tool set from server state. The host hands these to an
//! embedded awaken runtime; point tools share the store, the query tool the
//! DataFusion engine.
//!
//! A run may be confined to a tenant [`TenantScope`] (org/site). When scoped,
//! the point/board tools wrap their store access in [`ScopedPointAccess`] so a
//! read/write/history/board call outside the run's `{org}/{site}` is refused at
//! the tool boundary, `pin_widget` is gated to sites in the scope, and the
//! `query` SQL surface runs through a tenant-filtered DataFusion session
//! ([`QueryEngine::scoped_query`]) so the run's ad-hoc SQL can only read its own
//! `{org}/{site}`. An unscoped build (edge, or a run with no principal) keeps
//! today's full, open tool set with an unconstrained `query`.

use std::sync::Arc;

use awaken_runtime_contract::contract::tool::Tool;
use rubix_query::QueryScope;
use rubix_tools::{
    PinWidgetTool, QueryTool, ReadPointTool, RunBoardTool, ScopedPointAccess, TenantScope,
    WritePointTool,
};

use super::board_access::StoreBoardAccess;
use super::query_access::EngineQueryAccess;
use super::widget_access::StoreWidgetAccess;
use crate::flow::StorePointAccess;
use crate::AppState;
use rubix_tools::PointAccess;

/// Construct the agent-callable BMS tools, unscoped (every site reachable). Used
/// for the boot-time runtime on edge and as the fallback when a run carries no
/// tenant scope.
pub fn build_tools(state: &AppState) -> Vec<Arc<dyn Tool>> {
    build_tools_scoped(state, None)
}

/// Construct the BMS tools for a run, optionally confined to a tenant `scope`.
/// With a scope, point/board access is wrapped in [`ScopedPointAccess`],
/// `pin_widget` is gated to sites in the scope, and `query` runs through a
/// tenant-filtered DataFusion session.
pub fn build_tools_scoped(state: &AppState, scope: Option<TenantScope>) -> Vec<Arc<dyn Tool>> {
    let base: Arc<dyn PointAccess> = Arc::new(StorePointAccess::new(state.store.clone()));
    let access: Arc<dyn PointAccess> = match &scope {
        Some(scope) => Arc::new(ScopedPointAccess::new(base, scope.clone())),
        None => base,
    };
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ReadPointTool::new(access.clone())),
        Arc::new(WritePointTool::with_escalation_floor(
            access,
            state.ai_min_priority,
            state.ai_escalation_floor,
        )),
        Arc::new(RunBoardTool::new(Arc::new(StoreBoardAccess::scoped(
            state.store.clone(),
            scope.clone(),
        )))),
        Arc::new(PinWidgetTool::new(Arc::new(StoreWidgetAccess::scoped(
            state.store.clone(),
            scope.clone(),
        )))),
    ];
    // The `query` tool runs SQL through the DataFusion engine. An unscoped run
    // gets the full surface across every tenant; a scoped run gets a
    // tenant-filtered session (`scoped_query`) so its SQL can only read its own
    // `{org}/{site}`. A scope that cannot map to a `QueryScope` (empty or
    // quote-bearing org/site — impossible from a valid keyexpr) withholds the
    // tool rather than running it unscoped, keeping the boundary fail-closed.
    if let Some(engine) = &state.query {
        let query_access = match &scope {
            None => Some(EngineQueryAccess::new(engine.clone())),
            Some(scope) => QueryScope::new(scope.org(), scope.site())
                .ok()
                .map(|qs| EngineQueryAccess::scoped(engine.clone(), qs)),
        };
        if let Some(access) = query_access {
            tools.push(Arc::new(QueryTool::new(Arc::new(access))));
        }
    }
    tools
}
