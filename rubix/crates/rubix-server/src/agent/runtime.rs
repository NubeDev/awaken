//! Build the embedded awaken [`AgentRuntime`] that drives the BMS tools.
//!
//! The runtime registers the same `build_tools()` set the HTTP layer exposes,
//! so an LLM agent reads and commands points through the priority array and
//! queries history with the identical gating. Model/provider are env-selected;
//! the genai provider reads its API key from the environment at run time, not
//! at build, so a node with `RUBIX_AI=1` but no key still boots and only fails
//! when a chat turn actually calls the model.

use std::sync::Arc;

use awaken_runtime::engine::GenaiExecutor;
use awaken_runtime::{AgentRuntime, AgentRuntimeBuilder};
use awaken_runtime_contract::contract::executor::LlmExecutor;
use awaken_runtime_contract::{AgentSpec, ModelSpec};
use rubix_tools::TenantScope;

use crate::tools::build_tools_scoped;
use crate::AppState;

/// The agent id the chat endpoint activates. A single supervisory assistant
/// scoped to the building's points and history.
pub const AGENT_ID: &str = "rubix";

/// Default system prompt: a building-operations assistant constrained to the
/// tools. Operators tune model/prompt via env without code changes.
const SYSTEM_PROMPT: &str = "You are Rubix, a building management assistant. \
You read and command points and query history only through the provided tools. \
Commanding a point writes to its BACnet priority array; never command above the \
configured agent priority floor. Prefer reading current values and history \
before acting, and explain what you changed.";

/// Construct the embedded runtime from server state. `provider`/`model_id`/
/// `model` come from env (`RUBIX_AI_PROVIDER`, `RUBIX_AI_MODEL_ID`,
/// `RUBIX_AI_MODEL`); `max_rounds` bounds tool-call loops.
pub fn build_runtime(
    state: &AppState,
    provider: &str,
    model_id: &str,
    upstream_model: &str,
    max_rounds: usize,
) -> anyhow::Result<AgentRuntime> {
    build_runtime_with_executor(
        state,
        provider,
        model_id,
        upstream_model,
        max_rounds,
        Arc::new(GenaiExecutor::new()),
    )
}

/// Build the runtime with an explicit LLM executor. The public [`build_runtime`]
/// wires the genai provider; tests pass a scripted executor to exercise the
/// agent loop and BMS tools offline. Unscoped (full tool set).
pub fn build_runtime_with_executor(
    state: &AppState,
    provider: &str,
    model_id: &str,
    upstream_model: &str,
    max_rounds: usize,
    executor: Arc<dyn LlmExecutor>,
) -> anyhow::Result<AgentRuntime> {
    let blueprint =
        RuntimeBlueprint::with_executor(provider, model_id, upstream_model, max_rounds, executor);
    build_scoped_runtime(state, &blueprint, None)
}

/// The model/provider/executor inputs needed to build an [`AgentRuntime`], kept
/// so a tenant-scoped run can rebuild a runtime whose tools are confined to its
/// `{org}/{site}` (awaken 0.6 gives a tool no per-run scope context, so scope is
/// bound at tool-construction time — see [`build_scoped_runtime`]).
#[derive(Clone)]
pub struct RuntimeBlueprint {
    provider: String,
    model_id: String,
    upstream_model: String,
    max_rounds: usize,
    executor: Arc<dyn LlmExecutor>,
}

impl RuntimeBlueprint {
    /// Capture the blueprint from the resolved agent config and the genai
    /// provider executor (the boot path).
    pub fn genai(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        upstream_model: impl Into<String>,
        max_rounds: usize,
    ) -> Self {
        Self::with_executor(
            provider,
            model_id,
            upstream_model,
            max_rounds,
            Arc::new(GenaiExecutor::new()),
        )
    }

    /// Capture the blueprint with an explicit executor (tests inject a scripted
    /// one).
    pub fn with_executor(
        provider: impl Into<String>,
        model_id: impl Into<String>,
        upstream_model: impl Into<String>,
        max_rounds: usize,
        executor: Arc<dyn LlmExecutor>,
    ) -> Self {
        Self {
            provider: provider.into(),
            model_id: model_id.into(),
            upstream_model: upstream_model.into(),
            max_rounds,
            executor,
        }
    }
}

/// Build a runtime whose BMS tools are confined to `scope`. A `None` scope
/// yields the full, unscoped tool set (today's behavior); a `Some` scope wraps
/// point/board access in the tenant guard and withholds the cross-tenant query
/// surface. The same executor/model as the boot runtime is reused, so a scoped
/// run behaves identically save for the tenant confinement.
pub fn build_scoped_runtime(
    state: &AppState,
    blueprint: &RuntimeBlueprint,
    scope: Option<TenantScope>,
) -> anyhow::Result<AgentRuntime> {
    let mut builder = AgentRuntimeBuilder::new()
        .with_agent_spec(
            AgentSpec::new(AGENT_ID)
                .with_model_id(&blueprint.model_id)
                .with_system_prompt(SYSTEM_PROMPT)
                .with_max_rounds(blueprint.max_rounds),
        )
        .with_provider(&blueprint.provider, blueprint.executor.clone())
        .with_model(ModelSpec::new(
            &blueprint.model_id,
            &blueprint.provider,
            &blueprint.upstream_model,
        ));

    for tool in build_tools_scoped(state, scope) {
        let id = tool.descriptor().id.clone();
        builder = builder.with_tool(id, tool);
    }

    builder
        .build()
        .map_err(|e| anyhow::anyhow!("build scoped agent runtime: {e}"))
}
