//! `emit_spark` node: a rule board records a finding. Config gives `site` (the
//! `{org}/{site}` keyexpr prefix), `rule`, `severity` (`info`/`warning`/
//! `fault`, default `warning`), and an optional `message`.
//!
//! Two inports feed the finding text and severity:
//!
//! - `finding` — a structured `{ message, severity, value }` object from a
//!   upstream `rule` node. When connected, the rule's verdict is authoritative:
//!   its message and its severity drive the spark, overriding both config
//!   fields. This is the rule-node path — a rule's `finding("fault", …)` records
//!   a fault even if the node's static `severity` says otherwise.
//! - `value` — a scalar payload rendered as text, overriding the `message`
//!   config only. Severity stays config-driven. This is the legacy
//!   computed-string path.
//!
//! Sparks go through [`PointAccess::emit_spark`]; the host resolves the site
//! prefix to an id, persists the spark, and publishes it on the bus.

use std::collections::HashMap;
use std::sync::Arc;

use reflow_actor::message::Message;
use reflow_actor::ActorContext;
use rubix_core::SparkSeverity;

use super::actor_base::{config_str, error_out, ActorBase};
use crate::port::{PointAccess, SparkDraft};
use crate::rubix_node;

#[derive(Clone)]
pub struct EmitSparkActor {
    pub base: ActorBase,
    pub access: Arc<dyn PointAccess>,
    pub body: super::actor_base::NodeBody,
}

impl EmitSparkActor {
    pub fn new(access: Arc<dyn PointAccess>) -> Self {
        Self {
            base: ActorBase::new(&["value", "finding"], &["output", "error"]),
            access,
            body: Arc::new(emit),
        }
    }
}

fn emit(access: &Arc<dyn PointAccess>, context: &ActorContext) -> HashMap<String, Message> {
    let Some(site_prefix) = config_str(context, "site") else {
        return error_out("emit_spark: missing `site` config");
    };
    let Some(rule) = config_str(context, "rule") else {
        return error_out("emit_spark: missing `rule` config");
    };

    // A structured `finding` from a rule node is authoritative for both message
    // and severity; fall back to the `value`/config path otherwise.
    let (severity, message) = match finding_input(context) {
        Some(Ok(f)) => (f.severity, f.message),
        Some(Err(e)) => return error_out(e),
        None => {
            let severity = match severity_of(context) {
                Ok(s) => s,
                Err(e) => return error_out(e),
            };
            let message = match message_of(context) {
                Some(m) => m,
                None => {
                    return error_out("emit_spark: no `message` config and no `value`/`finding` input")
                }
            };
            (severity, message)
        }
    };
    let draft = SparkDraft {
        site_prefix,
        rule,
        severity,
        message,
    };
    match access.emit_spark(draft) {
        Ok(()) => HashMap::from([("output".to_string(), Message::Flow)]),
        Err(e) => error_out(format!("emit_spark: {e}")),
    }
}

/// A structured finding from the `finding` inport: the rule verdict's
/// authoritative message and (already-mapped) severity. Returns `None` when the
/// inport is not connected, `Some(Err)` when a connected payload is malformed.
fn finding_input(context: &ActorContext) -> Option<Result<StructuredFinding, String>> {
    let msg = context.get_payload().get("finding")?;
    let Message::Object(obj) = msg else {
        return Some(Err(format!(
            "emit_spark: `finding` input must be a rule verdict object, got {msg:?}"
        )));
    };
    let value = serde_json::Value::from(obj.as_ref().clone());
    Some(parse_finding(&value))
}

/// The rule-verdict shape the `finding` inport carries.
struct StructuredFinding {
    message: String,
    severity: SparkSeverity,
}

fn parse_finding(value: &serde_json::Value) -> Result<StructuredFinding, String> {
    let message = value
        .get("message")
        .and_then(|m| m.as_str())
        .ok_or("emit_spark: `finding` input missing string `message`")?
        .to_string();
    let severity = value
        .get("severity")
        .ok_or("emit_spark: `finding` input missing `severity`")?;
    let severity: SparkSeverity = serde_json::from_value(severity.clone())
        .map_err(|_| format!("emit_spark: invalid `finding` severity {severity}"))?;
    Ok(StructuredFinding { message, severity })
}

/// Severity from config, defaulting to `warning`. An unknown token is an error
/// rather than a silent default, so a typo surfaces.
fn severity_of(context: &ActorContext) -> Result<SparkSeverity, String> {
    match config_str(context, "severity") {
        None => Ok(SparkSeverity::Warning),
        Some(s) => serde_json::from_str::<SparkSeverity>(&format!("\"{s}\""))
            .map_err(|_| format!("emit_spark: invalid severity {s:?} (info|warning|fault)")),
    }
}

/// The finding text: the `value` inport payload if connected, else the
/// `message` config. The inport wins so a computed string overrides a static
/// one.
fn message_of(context: &ActorContext) -> Option<String> {
    if let Some(msg) = context.get_payload().get("value") {
        if let Some(text) = render_message(msg) {
            return Some(text);
        }
    }
    config_str(context, "message")
}

/// Render an inbound message as finding text. Scalars stringify; structured
/// payloads serialize to JSON so nothing is silently dropped.
fn render_message(msg: &Message) -> Option<String> {
    match msg {
        Message::String(s) => Some(s.as_ref().clone()),
        Message::Boolean(b) => Some(b.to_string()),
        Message::Float(n) => Some(n.to_string()),
        Message::Integer(i) => Some(i.to_string()),
        Message::Flow => None,
        other => serde_json::to_value(other)
            .ok()
            .map(|v| v.to_string()),
    }
}

rubix_node!(EmitSparkActor);
