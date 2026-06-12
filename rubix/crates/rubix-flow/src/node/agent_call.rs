//! `agent_call` node: a board asks the embedded agent to act on a situation.
//! Config gives the `prompt` (or it arrives on the `value` inport, which wins)
//! and an optional `thread` to group repeated calls (default `board-agent-call`).
//!
//! Fire-and-forget: the request is raised through [`PointAccess::request_agent`]
//! and the agent runs out-of-band — a control board must not block on an LLM.
//! The node acknowledges on `output` so the graph can continue. Boards run by
//! the agent's own `run_board` tool get a `PointAccess` without an agent, so the
//! request fails closed there and the agent → board → agent loop cannot recur.

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
    match access.request_agent(AgentRequest { thread, prompt }) {
        Ok(()) => HashMap::from([("output".to_string(), Message::Flow)]),
        Err(e) => error_out(format!("agent_call: {e}")),
    }
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
