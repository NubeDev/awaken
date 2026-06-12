//! `write_point` node: command a point's priority slot. The value arrives on
//! the `value` inport; keyexpr and priority come from node config (`point`,
//! `priority`, default 16). Always writes through the priority array.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::Message;
use reflow_actor::ActorContext;

use super::actor_base::{config_str, error_out, ActorBase};
use super::value_msg::{message_to_value, value_to_message};
use crate::port::PointAccess;
use crate::rubix_node;

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
            body: Arc::new(write),
        }
    }
}

fn write(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
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
    match access.write_point(&keyexpr, priority, value) {
        Ok(Some(v)) => HashMap::from([("output".to_string(), value_to_message(&v))]),
        Ok(None) => HashMap::from([("output".to_string(), Message::Flow)]),
        Err(e) => error_out(format!("write_point: {e}")),
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
