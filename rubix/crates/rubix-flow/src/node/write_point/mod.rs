//! `write_point` node: command a point's priority slot. The value arrives on
//! the `value` inport; keyexpr and priority come from node config (`point`,
//! `priority`, default 16). Always writes through the priority array.
//!
//! On the persistent scan engine the node is re-ticked every scan, so it
//! **coalesces unchanged commands**: it remembers the last `(priority, value)`
//! it committed in its actor state and skips a re-command when neither changed.
//! Re-asserting the same value to the same slot is a device-level no-op but would
//! otherwise spam the history/audit/bus every scan. A one-shot `run()` rebuilds
//! the network (fresh state), so Test-Run / on-demand / subscription always
//! command — the coalescing only suppresses the steady-state scan-loop churn.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::Message;
use reflow_actor::{ActorContext, MemoryState};
use rubix_core::PointValue;
use serde_json::{json, Value};

use super::actor_base::{boxed, config_str, error_out, ActorBase};
use super::value_msg::{message_to_value, value_to_message};
use crate::port::PointAccess;
use crate::rubix_node;

/// Actor-state key under which the last committed command is retained.
const LAST_COMMAND: &str = "write_point.last_command";

#[derive(Clone)]
pub struct WritePointActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: super::actor_base::NodeBody,
}

impl WritePointActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["value"], &["output", "error"]),
            access,
            body: Arc::new(|access, context| boxed(write(access, context))),
        }
    }
}

async fn write(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let Some(keyexpr) = config_str(context, "point") else {
        return error_out("write_point: missing `point` config");
    };
    let priority = priority_of(context);
    let Some(msg) = context.get_payload().get("value") else {
        return error_out("write_point: no `value` input");
    };
    let Some(value) = message_to_value(msg) else {
        return error_out("write_point: input is not a scalar point value");
    };

    // Coalesce: a re-command of the same value to the same slot is a no-op, so
    // skip it (and emit nothing) rather than re-pushing the priority array.
    let command = command_key(priority, &value);
    if last_command(context).as_ref() == Some(&command) {
        return HashMap::new();
    }

    match access.write_point(&keyexpr, priority, value).await {
        Ok(Some(v)) => {
            remember_command(context, command);
            HashMap::from([("output".to_string(), value_to_message(&v))])
        }
        Ok(None) => {
            remember_command(context, command);
            HashMap::from([("output".to_string(), Message::Flow)])
        }
        Err(e) => error_out(format!("write_point: {e}")),
    }
}

/// The committed-command fingerprint: priority + value. Stored in actor state so
/// the next scan can tell whether anything changed.
fn command_key(priority: u8, value: &PointValue) -> Value {
    json!({ "priority": priority, "value": serde_json::to_value(value).unwrap_or(Value::Null) })
}

/// The last command this actor committed, if any (retained across scans).
fn last_command(context: &ActorContext) -> Option<Value> {
    let state = context.get_state();
    let guard = state.lock();
    guard
        .as_any()
        .downcast_ref::<MemoryState>()
        .and_then(|mem| mem.get(LAST_COMMAND).cloned())
}

/// Retain `command` as the last committed command for coalescing.
fn remember_command(context: &ActorContext, command: Value) {
    let state = context.get_state();
    let mut guard = state.lock();
    if let Some(mem) = guard.as_mut_any().downcast_mut::<MemoryState>() {
        mem.insert(LAST_COMMAND, command);
    }
}

/// Priority slot from config, clamped to 1..=16; defaults to 16 (lowest).
fn priority_of(context: &ActorContext) -> u8 {
    context
        .get_config_hashmap()
        .get("priority")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(1, 16) as u8)
        .unwrap_or(16)
}

rubix_node!(WritePointActor);
