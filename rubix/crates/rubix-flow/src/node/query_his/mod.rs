//! `query_his` node: fetch recent history for a point (keyexpr from config
//! `point`, `limit` default 100), emit it as a JSON array on `output`. Rule
//! boards use this to evaluate trends.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::{EncodableValue, Message};
use reflow_actor::ActorContext;

use super::actor_base::{boxed, config_str, error_out, ActorBase};
use crate::port::PointAccess;
use crate::rubix_node;

#[derive(Clone)]
pub struct QueryHisActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: super::actor_base::NodeBody,
}

impl QueryHisActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["trigger"], &["output", "error"]),
            access,
            body: Arc::new(|access, context| boxed(query(access, context))),
        }
    }
}

async fn query(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let Some(keyexpr) = config_str(context, "point") else {
        return error_out("query_his: missing `point` config");
    };
    let limit = context
        .get_config_hashmap()
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;
    match access.query_his(&keyexpr, limit).await {
        Ok(samples) => match serde_json::to_value(&samples) {
            Ok(json) => HashMap::from([(
                "output".to_string(),
                Message::Object(Arc::new(EncodableValue::from(json))),
            )]),
            Err(e) => error_out(format!("query_his: encode {e}")),
        },
        Err(e) => error_out(format!("query_his: {e}")),
    }
}

rubix_node!(QueryHisActor);
