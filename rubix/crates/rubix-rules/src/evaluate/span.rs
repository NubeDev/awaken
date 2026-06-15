//! Build, emit, and persist the WS-08 span for one rule evaluation.
//!
//! The Rhai engine emits a span tree per evaluation — which sub-rules ran, the
//! values seen, and the decision — the deterministic answer to "why did this
//! fire" (`rubix/docs/SCOPE.md`, "Tracing"). This verb owns one node of that
//! tree: it composes a [`Span`] (root for the top rule, child under its parent
//! for a sub-rule) whose attributes record the rule id, the resolved inputs, the
//! invoked children, and the decision; it stamps the span's `trace_id` with the
//! evaluation's correlation id (contract #3, `rubix/STACK-DEISGN.md`); then it
//! emits the span on the WS-07 bus and appends it to the bounded WS-08 trace
//! table. The parent/child links are what make the persisted spans reassemble
//! into the evaluation tree via `rubix_trace::assemble_trace`.

use std::collections::HashMap;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_bus::ControlBus;
use rubix_core::{CorrelationId, Id};
use rubix_trace::{
    MetricsBuilder, SampleRate, Span, SpanStatus, emit_span, persist_span, upsert_summary,
};

use crate::engine::Decision;
use crate::error::{Result, RuleError};
use crate::rule::Rule;

/// The span name every rule-evaluation span carries.
pub const RULE_SPAN_NAME: &str = "rule.eval";

/// The identity links and timing of one evaluation span.
///
/// `span_id` is minted by the caller *before* the rule's sub-rules are evaluated,
/// so each sub-rule can link its span to this one as parent while this rule's own
/// span (which needs the final decision) is built afterwards — the two-phase that
/// keeps the tree correctly nested without a mutable span. `trace_id` is the
/// evaluation correlation id every node shares (contract #3,
/// `rubix/STACK-DEISGN.md`); `parent_span_id` is `None` for the root rule.
#[derive(Debug, Clone)]
pub struct SpanFrame {
    /// This span's own id, minted before its sub-rules are evaluated.
    pub span_id: Id,
    /// The evaluation correlation id every node of the tree shares.
    pub trace_id: CorrelationId,
    /// The parent span this one nests under; `None` for the root rule.
    pub parent_span_id: Option<Id>,
    /// Span start, epoch nanoseconds.
    pub start_ns: i64,
    /// Span end, epoch nanoseconds.
    pub end_ns: i64,
}

/// Compose the span for evaluating `rule` under `frame`'s identity links.
///
/// Attributes capture the rule id, the inputs the script saw, the sub-rules it
/// invoked, and the decision — the per-evaluation "why".
#[must_use]
pub fn build_span(
    rule: &Rule,
    frame: SpanFrame,
    inputs: &HashMap<String, f64>,
    child_values: &HashMap<String, f64>,
    decision: &Decision,
) -> Span {
    let mut attributes = serde_json::json!({
        "rule": rule.id.as_str(),
        "output": rule.output,
        "inputs": inputs,
        "subrules": child_values,
        "decision": {
            "fired": decision.fired,
            "value": decision.value,
            "reason": decision.reason,
        },
    });
    // Populate the §5a reserved metric keys the trace rollup folds out: a rule
    // evaluation is a `rule`-kind span, and reaching `build_span` means it
    // completed (an evaluation error never produces a decision), so its status is
    // `Ok`. Tokens/cost are left unset — rule evaluation has neither.
    MetricsBuilder::new()
        .kind("rule")
        .status(SpanStatus::Ok)
        .apply(&mut attributes);
    Span {
        span_id: frame.span_id,
        trace_id: frame.trace_id,
        parent_span_id: frame.parent_span_id,
        name: RULE_SPAN_NAME.to_owned(),
        attributes,
        start_ns: frame.start_ns,
        end_ns: frame.end_ns,
    }
}

/// Emit `span` on the bus, fold it into the trace summary, and append it to the
/// bounded trace table.
///
/// Emitting is fire-and-forget (a no-subscriber emit is a no-op, `rubix-trace`);
/// persistence is the durable, sampled copy that `assemble_trace` later reads
/// back. `trace_db` is the owner handle the trace table's append permission
/// requires (`rubix-trace`). Sampling may drop the span before the write — the
/// deliberate thinning that keeps traces bounded (contract #4) — which is not an
/// error.
///
/// The §5b `trace_summary` rollup is folded for **every** span, *before*
/// sampling, so the per-correlation-id summary (status/tokens/cost/span-count)
/// stays accurate even when span sampling thins the durable per-span copies — the
/// rollup is the cheap trace-list read, and sampling must not skew it.
///
/// # Errors
/// Returns [`RuleError::Span`] if the summary fold or the durable append fails.
pub async fn emit_and_persist(
    bus: &ControlBus,
    trace_db: &Surreal<Db>,
    namespace: &str,
    span: &Span,
    rate: SampleRate,
) -> Result<()> {
    emit_span(bus, span);
    upsert_summary(trace_db, namespace, span)
        .await
        .map_err(|e| RuleError::Span(e.to_string()))?;
    persist_span(trace_db, namespace, span, rate)
        .await
        .map_err(|e| RuleError::Span(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rubix_core::{CorrelationId, Id};

    use crate::engine::Decision;
    use crate::rule::Rule;

    use super::{build_span, SpanFrame, RULE_SPAN_NAME};

    fn decision() -> Decision {
        Decision {
            fired: true,
            value: 30.0,
            reason: "hot".to_owned(),
            scores: std::collections::BTreeMap::new(),
            group_id: None,
        }
    }

    #[test]
    fn a_root_span_carries_the_correlation_and_decision() {
        let rule = Rule::new(Id::from_raw("r"), "true", Vec::new(), "out");
        let corr = CorrelationId::carry("corr-1");
        let mut inputs = HashMap::new();
        inputs.insert("temp".to_owned(), 30.0_f64);
        let span = build_span(
            &rule,
            SpanFrame {
                span_id: Id::new(),
                trace_id: corr.clone(),
                parent_span_id: None,
                start_ns: 0,
                end_ns: 10,
            },
            &inputs,
            &HashMap::new(),
            &decision(),
        );
        assert!(span.is_root());
        assert_eq!(span.trace_id, corr);
        assert_eq!(span.name, RULE_SPAN_NAME);
        assert_eq!(span.attributes["rule"], "r");
        assert_eq!(span.attributes["decision"]["fired"], true);
        assert_eq!(span.attributes["inputs"]["temp"], 30.0);
        // The §5a reserved metric keys are populated for the rollup.
        assert_eq!(span.attributes["span.kind"], "rule");
        assert_eq!(span.attributes["span.status"], "ok");
    }

    #[test]
    fn a_child_span_links_to_its_parent() {
        let child_rule = Rule::new(Id::from_raw("child"), "true", Vec::new(), "out");
        let corr = CorrelationId::carry("corr-2");
        let parent_id = Id::new();
        let child = build_span(
            &child_rule,
            SpanFrame {
                span_id: Id::new(),
                trace_id: corr.clone(),
                parent_span_id: Some(parent_id.clone()),
                start_ns: 1,
                end_ns: 5,
            },
            &HashMap::new(),
            &HashMap::new(),
            &decision(),
        );
        assert!(!child.is_root());
        assert_eq!(child.trace_id, corr);
        assert_eq!(child.parent_span_id.as_ref(), Some(&parent_id));
    }
}
