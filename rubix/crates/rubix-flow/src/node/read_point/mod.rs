//! `read_point` node: read a point's current value by keyexpr (from node
//! config `point`), emit it on `output`. Fires on any input tick.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::Message;
use reflow_actor::ActorContext;

use super::actor_base::{config_str, error_out, ActorBase};
use super::value_msg::value_to_message;
use crate::port::PointAccess;
use crate::rubix_node;

#[derive(Clone)]
pub struct ReadPointActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: super::actor_base::NodeBody,
}

impl ReadPointActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["trigger"], &["output", "error"]),
            access,
            body: Arc::new(read),
        }
    }
}

fn read(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let Some(keyexpr) = config_str(context, "point") else {
        return error_out("read_point: missing `point` config");
    };
    match access.read_point(&keyexpr) {
        Ok(Some(v)) => HashMap::from([("output".to_string(), value_to_message(&v))]),
        Ok(None) => error_out(format!("read_point: `{keyexpr}` has no current value")),
        Err(e) => error_out(format!("read_point: {e}")),
    }
}

rubix_node!(ReadPointActor);
