//! Build the BMS agent tool set from server state. The host hands these to an
//! embedded awaken runtime; point tools share the store, the query tool the
//! DataFusion engine.
//!
//! A run may be confined to a tenant [`TenantScope`] (org/site). When scoped,
//! the point/board tools wrap their store access in [`ScopedPointAccess`] so a
//! read/write/history/board call outside the run's `{org}/{site}` is refused at
//! the tool boundary, `pin_widget` is gated to sites in the scope, and the
//! unconstrained `query` SQL surface is withheld (a tenant-aware query view is a
//! follow-up — see docs/sessions/TODOs.md). An unscoped build (edge, or a run
//! with no principal) keeps today's full, open tool set.

use std::sync::Arc;

use awaken_runtime_contract::contract::tool::Tool;
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
/// `pin_widget` is gated to sites in the scope, and `query` is withheld.
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
    // The DataFusion `query` surface runs unconstrained SQL across every tenant's
    // tables; it has no per-tenant view today, so a tenant-scoped run does not
    // get it (fail-closed) rather than leaking cross-tenant rows.
    if scope.is_none() {
        if let Some(engine) = &state.query {
            let query_access = Arc::new(EngineQueryAccess::new(engine.clone()));
            tools.push(Arc::new(QueryTool::new(query_access)));
        }
    }
    tools
}
