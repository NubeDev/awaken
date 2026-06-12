//! `agent_call` node: a board asks the embedded agent to act on a situation.
//! Config gives the `prompt` (or it arrives on the `value` inport, which wins)
//! and an optional `thread` to group repeated calls (default `board-agent-call`).
//!
//! Two modes, selected by the `await` config flag (default `false`):
//!
//! - Detached (default): the request is raised through
//!   [`PointAccess::request_agent`] and the agent runs out-of-band — a control
//!   board must not block on an LLM. The node acknowledges on `output` so the
//!   graph continues immediately.
//! - Awaited (`await: true`): the node blocks on
//!   [`PointAccess::request_agent_blocking`] and emits the agent's final response
//!   text on `output`, so a downstream node can branch on the agent's decision
//!   in the same single-shot board run.
//!
//! Boards run by the agent's own `run_board` tool get a `PointAccess` without an
//! agent, so the request fails closed there and the agent → board → agent loop
//! cannot recur, in either mode.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::Message;
use reflow_actor::ActorContext;

use super::actor_base::{config_str, error_out, ActorBase};
use super::value_msg::message_to_value;
use crate::port::{AgentRequest, PointAccess};
use crate::rubix_node;

/// Default agent thread when the node does not name one.
const DEFAULT_THREAD: &str = "board-agent-call";

#[derive(Clone)]
pub struct AgentCallActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: super::actor_base::NodeBody,
}

impl AgentCallActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["value"], &["output", "error"]),
            access,
            body: Arc::new(call),
        }
    }
}

fn call(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let prompt = match prompt_of(context) {
        Some(p) => p,
        None => return error_out("agent_call: no `prompt` config and no `value` input"),
    };
    let thread = config_str(context, "thread").unwrap_or_else(|| DEFAULT_THREAD.to_string());
    let request = AgentRequest { thread, prompt };
    if awaits(context) {
        match access.request_agent_blocking(request) {
            Ok(outcome) => HashMap::from([(
                "output".to_string(),
                Message::String(outcome.response.into()),
            )]),
            Err(e) => error_out(format!("agent_call: {e}")),
        }
    } else {
        match access.request_agent(request) {
            Ok(()) => HashMap::from([("output".to_string(), Message::Flow)]),
            Err(e) => error_out(format!("agent_call: {e}")),
        }
    }
}

/// Whether the node blocks on the run and surfaces its outcome (`await: true`),
/// or fires it detached (the control-board default).
fn awaits(context: &ActorContext) -> bool {
    context
        .get_config_hashmap()
        .get("await")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// The prompt: the `value` inport rendered as text if connected, else the
/// `prompt` config. The inport wins so an upstream-computed prompt overrides a
/// static one (symmetry with `emit_spark`'s message).
fn prompt_of(context: &ActorContext) -> Option<String> {
    if let Some(msg) = context.get_payload().get("value") {
        if let Some(value) = message_to_value(msg) {
            return Some(match value {
                rubix_core::PointValue::Str(s) => s,
                rubix_core::PointValue::Bool(b) => b.to_string(),
                rubix_core::PointValue::Number(n) => n.to_string(),
            });
        }
    }
    config_str(context, "prompt")
}

rubix_node!(AgentCallActor);
