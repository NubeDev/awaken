//! Map board component names to rubix actor instances. Every node sharing a
//! component name gets its own actor instance (own channels) but the same
//! injected [`PointAccess`].

use std::sync::Arc;

use reflow_actor::Actor;

use crate::node::{
    AgentCallActor, EmitSparkActor, QueryHisActor, ReadPointActor, WritePointActor,
};
use crate::port::PointAccess;

/// Built-in rubix component names available to boards.
pub const COMPONENTS: [&str; 5] = [
    "read_point",
    "write_point",
    "query_his",
    "emit_spark",
    "agent_call",
];

/// Construct the actor for a component name, or `None` if unknown.
pub fn make_actor(component: &str, access: Arc<dyn PointAccess>) -> Option<Arc<dyn Actor>> {
    match component {
        "read_point" => Some(Arc::new(ReadPointActor::new(access))),
        "write_point" => Some(Arc::new(WritePointActor::new(access))),
        "query_his" => Some(Arc::new(QueryHisActor::new(access))),
        "emit_spark" => Some(Arc::new(EmitSparkActor::new(access))),
        "agent_call" => Some(Arc::new(AgentCallActor::new(access))),
        _ => None,
    }
}
