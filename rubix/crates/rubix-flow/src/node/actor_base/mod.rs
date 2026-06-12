//! Shared channel plumbing for rubix actors. reflow's `#[actor]` macro can't
//! capture injected dependencies, so rubix nodes hand-implement [`Actor`] over
//! this base, which owns the inport/outport channels and the port-name lists.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::Message;
use reflow_actor::{ActorContext, Port};

use crate::port::PointAccess;

/// Channels + declared ports for one node. The cross-assignment of capacities
/// mirrors the macro: inports receive the outport-declared capacity. We use
/// unbounded channels (control boards are low-rate), so the swap is moot.
#[derive(Clone)]
pub struct ActorBase {
    pub inports: Port,
    pub outports: Port,
    pub inport_names: Vec<String>,
    pub outport_names: Vec<String>,
}

impl ActorBase {
    pub fn new(inport_names: &[&str], outport_names: &[&str]) -> Self {
        Self {
            inports: flume::unbounded(),
            outports: flume::unbounded(),
            inport_names: inport_names.iter().map(|s| s.to_string()).collect(),
            outport_names: outport_names.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// The behavior body a rubix node provides: given its [`PointAccess`] and the
/// invocation context, produce the outport payload.
pub type NodeBody =
    Arc<dyn Fn(&Arc<dyn PointAccess>, &ActorContext) -> HashMap<String, Message> + Send + Sync>;

/// Implement [`Actor`] for a node struct that exposes its [`ActorBase`], its
/// `Arc<dyn PointAccess>`, and a [`NodeBody`]. Keeps each node file to just its
/// ports and logic.
#[macro_export]
macro_rules! rubix_node {
    ($ty:ty) => {
        impl reflow_actor::Actor for $ty {
            fn get_behavior(&self) -> reflow_actor::ActorBehavior {
                let access = self.access.clone();
                let body = self.body.clone();
                Box::new(move |context: reflow_actor::ActorContext| {
                    let access = access.clone();
                    let body = body.clone();
                    Box::pin(async move { Ok(body(&access, &context)) })
                })
            }
            fn get_outports(&self) -> reflow_actor::Port {
                self.base.outports.clone()
            }
            fn get_inports(&self) -> reflow_actor::Port {
                self.base.inports.clone()
            }
            fn inport_names(&self) -> Vec<String> {
                self.base.inport_names.clone()
            }
            fn outport_names(&self) -> Vec<String> {
                self.base.outport_names.clone()
            }
            fn create_instance(&self) -> std::sync::Arc<dyn reflow_actor::Actor> {
                std::sync::Arc::new(self.clone())
            }
        }
    };
}

/// Read a string config/metadata value from the node's `ActorConfig`.
pub fn config_str(context: &ActorContext, key: &str) -> Option<String> {
    context
        .get_config_hashmap()
        .get(key)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Standard error output on the `error` port.
pub fn error_out(msg: impl Into<String>) -> HashMap<String, Message> {
    let mut out = HashMap::new();
    out.insert("error".to_string(), Message::Error(msg.into().into()));
    out
}
