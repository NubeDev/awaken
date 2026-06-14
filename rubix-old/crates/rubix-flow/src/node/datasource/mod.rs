//! `datasource` node: run a read-only query against an external SQL datasource
//! (a TimescaleDB/Postgres historian) and emit `{ columns, rows, breached }` on
//! `output`, the same shape `query_his` emits so a downstream `rule` node folds
//! it identically.
//!
//! Config:
//! - `datasource` (required) — the registered datasource id.
//! - `sql` *or* `named` (exactly one) — operator-authored native SQL, or the
//!   name of an operator-registered named query. Both are board-authored, never
//!   end-user input (docs/design/datasources.md "Query authoring tiers").
//! - `params` (optional) — a JSON array of typed bound parameters
//!   (`[{type,value}, …]`), bound positionally, never spliced into SQL.
//!
//! This is the spark path: the host runs it under the *strict* cap policy, so a
//! result that breaches the datasource's caps fails the node (the `error` port)
//! rather than emitting a truncated grid a finding could misread
//! (docs/design/datasources.md "Truncation on the spark path").

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::{EncodableValue, Message};
use reflow_actor::ActorContext;
use serde_json::Value;

use super::actor_base::{boxed, error_out, ActorBase};
use crate::port::{DatasourceQuery, PointAccess};
use crate::rubix_node;

#[derive(Clone)]
pub struct DatasourceActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: super::actor_base::NodeBody,
}

impl DatasourceActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["trigger"], &["output", "error"]),
            access,
            body: Arc::new(|access, context| boxed(query(access, context))),
        }
    }
}

/// One resolved `datasource` node request: the id, the query intent, and the
/// (positional) bound parameters. Parsed from config before the host runs it.
#[derive(Debug)]
struct Request {
    datasource: String,
    intent: DatasourceQuery,
    params: Value,
}

async fn query(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let req = match parse_request(&context.get_config_hashmap()) {
        Ok(req) => req,
        Err(e) => return error_out(e),
    };
    match access
        .query_datasource(&req.datasource, req.intent, req.params)
        .await
    {
        Ok(result) => HashMap::from([(
            "output".to_string(),
            Message::Object(Arc::new(EncodableValue::from(result))),
        )]),
        Err(e) => error_out(format!("datasource: {e}")),
    }
}

/// Resolve the node's config into a [`Request`], or a configuration-error
/// message the node surfaces on `error`. Requires `datasource` and exactly one
/// of `sql` / `named`; `params` defaults to an empty array. Kept config-map
/// shaped (not `ActorContext` shaped) so the parsing is unit-testable directly.
fn parse_request(config: &HashMap<String, Value>) -> Result<Request, String> {
    let datasource = config_str(config, "datasource")
        .ok_or("datasource: missing `datasource` (datasource id) config")?;
    let intent = query_intent(config)?;
    let params = config
        .get("params")
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    Ok(Request {
        datasource,
        intent,
        params,
    })
}

/// Resolve the query intent: exactly one of `sql` or `named`. Both or neither is
/// a configuration error the node surfaces, not a silent default.
fn query_intent(config: &HashMap<String, Value>) -> Result<DatasourceQuery, String> {
    match (config_str(config, "sql"), config_str(config, "named")) {
        (Some(_), Some(_)) => {
            Err("datasource: set exactly one of `sql` or `named`, not both".into())
        }
        (Some(sql), None) => Ok(DatasourceQuery::Sql(sql)),
        (None, Some(named)) => Ok(DatasourceQuery::Named(named)),
        (None, None) => Err("datasource: set one of `sql` (native SQL) or `named` \
             (a registered named query)"
            .into()),
    }
}

/// A non-empty string config value, or `None`.
fn config_str(config: &HashMap<String, Value>, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

rubix_node!(DatasourceActor);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config(v: Value) -> HashMap<String, Value> {
        v.as_object()
            .map(|o| o.clone().into_iter().collect())
            .unwrap_or_default()
    }

    #[test]
    fn missing_datasource_is_a_config_error() {
        let err = parse_request(&config(json!({ "sql": "SELECT 1" }))).unwrap_err();
        assert!(err.contains("datasource"));
    }

    #[test]
    fn both_sql_and_named_is_a_config_error() {
        let err = parse_request(&config(
            json!({ "datasource": "h", "sql": "SELECT 1", "named": "daily" }),
        ))
        .unwrap_err();
        assert!(err.contains("not both"));
    }

    #[test]
    fn neither_sql_nor_named_is_a_config_error() {
        let err = parse_request(&config(json!({ "datasource": "h" }))).unwrap_err();
        assert!(err.contains("native SQL"));
    }

    #[test]
    fn raw_sql_resolves_to_sql_intent_with_params() {
        let req = parse_request(&config(json!({
            "datasource": "historian",
            "sql": "SELECT $1::int",
            "params": [{"type":"int","value":7}]
        })))
        .unwrap();
        assert_eq!(req.datasource, "historian");
        assert_eq!(req.intent, DatasourceQuery::Sql("SELECT $1::int".into()));
        assert_eq!(req.params, json!([{"type":"int","value":7}]));
    }

    #[test]
    fn named_resolves_to_named_intent_with_default_params() {
        let req = parse_request(&config(
            json!({ "datasource": "h", "named": "site_daily" }),
        ))
        .unwrap();
        assert_eq!(req.intent, DatasourceQuery::Named("site_daily".into()));
        assert_eq!(req.params, json!([]), "params default to an empty array");
    }
}
