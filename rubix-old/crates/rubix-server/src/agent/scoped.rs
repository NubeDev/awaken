//! Resolve the agent runtime a run executes on for its tenant scope. A run with
//! no scope (edge, or an unauthenticated caller) uses the shared boot runtime
//! with the full tool set; a tenant-scoped run gets a freshly built runtime
//! whose BMS tools are confined to its `{org}/{site}` (awaken 0.6 binds tool
//! scope at construction time — see [`super::build_scoped_runtime`]).
//!
//! STACK-DEISGN.md "Tenancy: org/site hierarchy mirrors into awaken `ScopeId`".

use std::sync::Arc;

use awaken_runtime::AgentRuntime;
use rubix_tools::TenantScope;

use super::build_scoped_runtime;
use crate::AppState;

/// The runtime a run should use. `None` scope returns the shared unscoped
/// runtime; a `Some` scope builds a tenant-confined runtime. Returns `None` when
/// the agent is disabled (no runtime and no blueprint).
pub fn runtime_for_scope(
    state: &AppState,
    scope: Option<TenantScope>,
) -> anyhow::Result<Option<Arc<AgentRuntime>>> {
    match scope {
        None => Ok(state.agent.clone()),
        Some(scope) => match &state.agent_blueprint {
            Some(blueprint) => {
                let runtime = build_scoped_runtime(state, blueprint, Some(scope))?;
                Ok(Some(Arc::new(runtime)))
            }
            // The agent is off: no blueprint to scope. Surface as "no runtime"
            // so callers return the same unavailable response as an unscoped
            // run, never silently dropping the tenant confinement.
            None => Ok(None),
        },
    }
}
