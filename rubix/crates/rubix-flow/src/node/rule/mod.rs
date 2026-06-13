//! `rule` node: the board graph's compute step, between a query node and
//! `emit_spark`.
//!
//! It takes the upstream query result (a `query_his` JSON sample array) on its
//! `input` inport as the rule [`Frame`], runs an inline or stored Rhai rule over
//! it via [`rubix_rules::run_rule`], and turns the verdict into board output:
//!
//! - **flagged** → a structured finding `{ message, severity }` on `finding`,
//!   wired to `emit_spark`'s `value` inport. The rule's severity is
//!   authoritative (mapped to the canonical [`rubix_core::SparkSeverity`]); a
//!   downstream `emit_spark` records it rather than its static config severity.
//! - **clear** (ran, found nothing) → a `Flow` tick on `clear`. This is a normal
//!   non-emit, not an error: nothing reaches `emit_spark`.
//! - **error** — a [`rubix_rules::RuleError`] (broken rule / composition
//!   failure) or an input caps breach (truncated input) fails the node on
//!   `error`, never emitting a finding from partial or broken data.
//!
//! Config: `script` (inline Rhai) or `rule` (a stored rule id, resolved through
//! the [`PointAccess::rule_store`]); exactly one is required. `params` is a JSON
//! object exposed to the script as `params`. `max_rows` caps the input frame
//! (default [`DEFAULT_MAX_ROWS`]).

mod frame;
mod severity;

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::{EncodableValue, Message};
use reflow_actor::ActorContext;
use rubix_rules::{params_from_json, run_rule, RuleResult, RuleSource, SandboxLimits};

use self::frame::{frame_from_samples, FrameError};
use self::severity::spark_severity;
use super::actor_base::{config_str, error_out, ActorBase};
use crate::port::PointAccess;
use crate::rubix_node;

pub use self::severity::spark_severity as map_severity;

/// Default cap on the input frame: a rule folds a bounded, query-capped window,
/// not an unbounded historian read. An input larger than this is treated as a
/// truncation breach (see [`frame`]).
pub const DEFAULT_MAX_ROWS: usize = 10_000;

#[derive(Clone)]
pub struct RuleActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: super::actor_base::NodeBody,
}

impl RuleActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["input"], &["finding", "clear", "error"]),
            access,
            body: Arc::new(evaluate),
        }
    }
}

fn evaluate(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let payload = match input_payload(context) {
        Ok(p) => p,
        Err(e) => return error_out(e),
    };
    let max_rows = config_usize(context, "max_rows").unwrap_or(DEFAULT_MAX_ROWS);
    let frame = match frame_from_samples(&payload, max_rows) {
        Ok(f) => f,
        Err(FrameError::CapsBreach { rows, max_rows }) => {
            // Truncated input must not be folded into a finding — fail the node.
            return error_out(format!("rule: {}", FrameError::CapsBreach { rows, max_rows }));
        }
        Err(e) => return error_out(format!("rule: {e}")),
    };

    let params = params_from_json(&config_json(context, "params"));
    let limits = SandboxLimits::default();

    // Resolve the source. A stored rule needs the tenant store; an inline script
    // does not. The store is still required to construct the executor (it backs
    // `rule(name, …)` composition), so a board referencing stored rules without a
    // store-backed access fails closed rather than silently skipping composition.
    let store = access.rule_store();
    let result = match source_config(context) {
        Ok(SourceKind::Inline(script)) => {
            let store = store.unwrap_or_else(rubix_rules_empty_store);
            run_blocking(|| run_rule(store, RuleSource::Inline(&script), frame, params, limits))
        }
        Ok(SourceKind::Stored(id)) => match store {
            Some(store) => {
                run_blocking(|| run_rule(store, RuleSource::Stored(&id), frame, params, limits))
            }
            None => return error_out("rule: `rule` id set but this board has no rule store"),
        },
        Err(e) => return error_out(e),
    };

    match result {
        Ok(verdict) => emit_verdict(verdict),
        // A RuleError fails the node — a broken rule is an operational error,
        // distinct from a clear (non-flagged) result, which is a normal no-emit.
        Err(e) => error_out(format!("rule: {e}")),
    }
}

/// Turn a verdict into outport messages: a structured finding on `finding` when
/// flagged, else a `Flow` tick on `clear`.
fn emit_verdict(verdict: RuleResult) -> HashMap<String, Message> {
    if !verdict.flagged {
        return HashMap::from([("clear".to_string(), Message::Flow)]);
    }
    let finding = serde_json::json!({
        "message": verdict.message,
        "severity": spark_severity(verdict.severity),
        "value": verdict.value,
    });
    HashMap::from([(
        "finding".to_string(),
        Message::Object(Arc::new(EncodableValue::from(finding))),
    )])
}

/// The `input` payload as JSON. The query node emits an `Object` (a sample
/// array); anything else is a wiring error the node fails on.
fn input_payload(context: &ActorContext) -> Result<serde_json::Value, String> {
    let Some(msg) = context.get_payload().get("input") else {
        return Err("rule: no `input` payload (wire a query node into `input`)".into());
    };
    match msg {
        Message::Object(obj) => Ok(serde_json::Value::from(obj.as_ref().clone())),
        Message::Array(arr) => Ok(serde_json::Value::Array(
            arr.iter()
                .map(|ev| serde_json::Value::from(ev.clone()))
                .collect(),
        )),
        other => Err(format!(
            "rule: `input` must be a query result object/array, got {other:?}"
        )),
    }
}

enum SourceKind {
    Inline(String),
    Stored(String),
}

/// The rule source from config: exactly one of `script` (inline) or `rule`
/// (stored id). Both or neither is a configuration error.
fn source_config(context: &ActorContext) -> Result<SourceKind, String> {
    let script = config_str(context, "script");
    let stored = config_str(context, "rule");
    match (script, stored) {
        (Some(s), None) => Ok(SourceKind::Inline(s)),
        (None, Some(id)) => Ok(SourceKind::Stored(id)),
        (Some(_), Some(_)) => Err("rule: set exactly one of `script` or `rule`, not both".into()),
        (None, None) => Err("rule: missing `script` (inline) or `rule` (stored id)".into()),
    }
}

fn config_usize(context: &ActorContext, key: &str) -> Option<usize> {
    context
        .get_config_hashmap()
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
}

fn config_json(context: &ActorContext, key: &str) -> serde_json::Value {
    context
        .get_config_hashmap()
        .get(key)
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

/// Evaluate a rule, bridging the sync/blocking engine into the async actor
/// task.
///
/// `rubix_rules` is synchronous and drives DataFusion on a fresh current-thread
/// runtime per primitive (`block_on`). The board actor runs inside a Tokio
/// worker, where a nested `block_on` would panic ("cannot start a runtime from
/// within a runtime"). [`tokio::task::block_in_place`] hands the worker to the
/// blocking call so the engine's runtime can drive to completion; without a
/// multi-thread runtime (unit contexts) the engine is called directly.
fn run_blocking<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread => {
            tokio::task::block_in_place(f)
        }
        _ => f(),
    }
}

/// An empty rule store for inline scripts that do not compose stored rules. A
/// `rule(name, …)` against it fails closed with a resolve error.
fn rubix_rules_empty_store() -> Arc<dyn rubix_rules::RuleStore> {
    Arc::new(rubix_rules::MemoryRuleStore::new())
}

rubix_node!(RuleActor);
