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

use crate::tools::build_tools;
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
/// agent loop and BMS tools offline.
pub fn build_runtime_with_executor(
    state: &AppState,
    provider: &str,
    model_id: &str,
    upstream_model: &str,
    max_rounds: usize,
    executor: Arc<dyn LlmExecutor>,
) -> anyhow::Result<AgentRuntime> {
    let mut builder = AgentRuntimeBuilder::new()
        .with_agent_spec(
            AgentSpec::new(AGENT_ID)
                .with_model_id(model_id)
                .with_system_prompt(SYSTEM_PROMPT)
                .with_max_rounds(max_rounds),
        )
        .with_provider(provider, executor)
        .with_model(ModelSpec::new(model_id, provider, upstream_model));

    for tool in build_tools(state) {
        let id = tool.descriptor().id.clone();
        builder = builder.with_tool(id, tool);
    }

    builder
        .build()
        .map_err(|e| anyhow::anyhow!("build agent runtime: {e}"))
}
