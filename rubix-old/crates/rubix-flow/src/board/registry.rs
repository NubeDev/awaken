//! Map board component names to rubix actor instances. Every node sharing a
//! component name gets its own actor instance (own channels) but the same
//! injected [`PointAccess`].

use std::sync::Arc;

use reflow_actor::Actor;

use crate::node::{
    AgentCallActor, DatasourceActor, EmitSparkActor, QueryHisActor, ReadPointActor, RuleActor,
    TriggerActor, WritePointActor,
};
use crate::port::PointAccess;

/// Built-in rubix component names available to boards.
pub const COMPONENTS: [&str; 8] = [
    "read_point",
    "write_point",
    "query_his",
    "datasource",
    "rule",
    "emit_spark",
    "agent_call",
    "trigger",
];

/// Construct the actor for a component name, or `None` if unknown.
pub fn make_actor(component: &str, access: Arc<dyn PointAccess>) -> Option<Arc<dyn Actor>> {
    match component {
        "read_point" => Some(Arc::new(ReadPointActor::new(access))),
        "write_point" => Some(Arc::new(WritePointActor::new(access))),
        "query_his" => Some(Arc::new(QueryHisActor::new(access))),
        "datasource" => Some(Arc::new(DatasourceActor::new(access))),
        "rule" => Some(Arc::new(RuleActor::new(access))),
        "emit_spark" => Some(Arc::new(EmitSparkActor::new(access))),
        "agent_call" => Some(Arc::new(AgentCallActor::new(access))),
        "trigger" => Some(Arc::new(TriggerActor::new(access))),
        _ => None,
    }
}
