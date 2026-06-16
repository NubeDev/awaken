//! Evaluate a rule and its sub-rules depth-first, building the span tree.
//!
//! Rules are composable — a rule invokes another rule (`rubix/docs/SCOPE.md`,
//! "Rhai — rules and insights"). This verb is the recursive core of that: to
//! evaluate a rule it mints the rule's span id, evaluates each declared sub-rule
//! depth-first under that span id (so a child's decision exists, and its span
//! nests, before the parent's script runs), exposes each child's decision value
//! to the parent's `invoke`, resolves the parent's own window-value bindings, runs
//! the parent script, and emits/persists the parent's span. Every node shares the
//! one evaluation correlation id, so the persisted spans reassemble into a single
//! tree that shows the full "why" — each sub-rule, the values it saw, and its
//! decision (contract #3, `rubix/STACK-DEISGN.md`).

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use rubix_core::Id;

use crate::engine::Decision;
use crate::error::Result;
use crate::rule::{Rule, resolve};

use super::clock::now_ns;
use super::context::EvalContext;
use super::run::run_script;
use super::span::{SpanFrame, build_span, emit_and_persist};

/// Recursively evaluate `rule`, nesting its span under `parent_span_id`.
///
/// Sub-rules are evaluated first and their decision values fed to this rule's
/// `invoke`; this rule's bindings are then resolved through the scoped session and
/// its script run. The span (root when `parent_span_id` is `None`) is emitted and
/// persisted, and `assemble_trace` later links the forest by parent id.
///
/// `Box`ed future because the recursion is over an async fn (a rule composing a
/// rule), which an `async fn` cannot express directly.
///
/// # Errors
/// Propagates any binding, evaluation, or span-persist failure from this rule or
/// any sub-rule (fail closed — a missing sub-rule or unresolved window value
/// fails the whole evaluation rather than producing a partial decision).
pub fn evaluate_rule<'a>(
    ctx: &'a EvalContext<'a>,
    rule: &'a Rule,
    parent_span_id: Option<Id>,
) -> Pin<Box<dyn Future<Output = Result<Decision>> + 'a>> {
    Box::pin(async move {
        let start_ns = now_ns();
        let span_id = Id::new();

        // Depth-first: evaluate each declared sub-rule under this rule's span id,
        // collecting its decision value for this rule's `invoke`.
        let mut child_values: HashMap<String, f64> = HashMap::new();
        for sub_id in &rule.subrules {
            let sub = ctx.registry.resolve(sub_id.as_str())?;
            let decision = evaluate_rule(ctx, sub, Some(span_id.clone())).await?;
            child_values.insert(sub.id.as_str().to_owned(), decision.value);
        }

        // Resolve this rule's own window-value inputs through the scoped session.
        let mut inputs: HashMap<String, f64> = HashMap::new();
        for binding in &rule.inputs {
            let value = resolve(ctx.session, binding).await?;
            inputs.insert(binding.name.clone(), value);
        }

        let decision = run_script(rule, &inputs, child_values.clone())?;
        let end_ns = now_ns();

        let span = build_span(
            rule,
            SpanFrame {
                span_id,
                trace_id: ctx.correlation.clone(),
                parent_span_id,
                start_ns,
                end_ns,
            },
            &inputs,
            &child_values,
            &decision,
        );
        emit_and_persist(ctx.bus, ctx.trace_db, ctx.namespace(), &span, ctx.sample).await?;

        Ok(decision)
    })
}
