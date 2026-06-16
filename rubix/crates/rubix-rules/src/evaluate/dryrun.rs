//! Run a rule once with no side effects — the edit→run debugger loop.
//!
//! [`evaluate`](super::evaluate) is the production path: it records the insight
//! through the gate, publishes the firing, and persists a span tree. A *dry-run*
//! is the opposite contract — author a rule and see what it would decide against
//! real history *without* recording an insight, crossing the command gate, or
//! emitting a trace. It resolves the same window values [`evaluate`] does (through
//! the principal's scoped session, so row-level permissions still decide the rows,
//! contract #1) and runs the same [`run_script`], so the verdict is faithful; it
//! simply stops there. Sub-rules a rule composes are resolved depth-first the same
//! way, also side-effect-free.
//!
//! Beyond the [`Decision`], a dry-run returns the [`BucketRollup`]s each binding
//! resolved from, so a debugger can chart the exact frame the rule saw — the
//! deterministic "this is the window it decided on" without re-scanning.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_query::{BucketRollup, SeriesFilter, rollup_window_filtered};

use crate::engine::Decision;
use crate::error::{Result, RuleError};
use crate::evaluate::run::run_script;
use crate::rule::{Binding, Rule, RuleRegistry};

/// One binding's resolved window: the buckets it rolled up and the value selected.
///
/// The buckets are the exact frame the rule saw for this input (ascending by
/// `bucket_start`); `value` is the binding's aggregate of the most recent bucket —
/// the number bound into the script. A debugger charts `buckets` and labels the
/// fired point with `value`.
#[derive(Debug, Clone)]
pub struct ResolvedInput {
    /// The script variable name this input bound to.
    pub name: String,
    /// The window buckets the binding rolled up, ascending by start.
    pub buckets: Vec<BucketRollup>,
    /// The aggregate value selected from the latest bucket — what the script read.
    pub value: f64,
}

/// A rule's dry-run result before it is wrapped: its decision and resolved inputs.
///
/// Named because the recursive [`run_dry`] returns it boxed, and a bare boxed
/// tuple future is the `clippy::type_complexity` lint — the alias keeps the
/// recursion readable.
type DryOutcome = (Decision, Vec<ResolvedInput>);

/// The future [`run_dry`] returns — boxed because the recursion is over an async
/// fn, and `Send` so the dry-run can drive from an axum handler.
type DryFuture<'a> = Pin<Box<dyn Future<Output = Result<DryOutcome>> + Send + 'a>>;

/// The outcome of a side-effect-free dry-run: the decision and the frame it saw.
///
/// `decision` is exactly what [`run_script`] produced; `inputs` carries the
/// resolved window for each binding so the caller can show the frame the rule
/// decided on. Nothing here was recorded, published, or traced.
#[derive(Debug, Clone)]
pub struct DryRun {
    /// The decision the rule's script produced.
    pub decision: Decision,
    /// The resolved window for each of the rule's bindings, in declaration order.
    pub inputs: Vec<ResolvedInput>,
}

/// Evaluate `root_id` in `registry` against `session` with no side effects.
///
/// Resolves each sub-rule depth-first (for `invoke`) and each of the root rule's
/// bindings through the scoped `session`, runs the root script, and returns the
/// [`Decision`] together with the buckets each binding resolved from. No insight
/// is recorded, no event is published, and no span is persisted — this is the
/// debugger's tight loop, not a firing.
///
/// # Errors
/// Returns a [`RuleError`] if the root rule or a sub-rule is unknown, a binding
/// cannot resolve to a window value, or the script fails — fail closed, the same
/// way [`evaluate`](super::evaluate) does (no silent zero for missing data).
pub async fn dry_run(
    session: &Surreal<Db>,
    registry: &RuleRegistry,
    root_id: &str,
) -> Result<DryRun> {
    let root = registry.resolve(root_id)?;
    let (decision, inputs) = run_dry(session, registry, root).await?;
    Ok(DryRun { decision, inputs })
}

/// Resolve `rule`'s sub-rules and bindings against `session` and run its script,
/// returning the decision and the buckets each binding resolved from.
///
/// `Box`ed future because the recursion is over an async fn (a rule composing a
/// rule), which an `async fn` cannot express directly — mirrors
/// [`evaluate_rule`](super::compose::evaluate_rule).
fn run_dry<'a>(
    session: &'a Surreal<Db>,
    registry: &'a RuleRegistry,
    rule: &'a Rule,
) -> DryFuture<'a> {
    Box::pin(async move {
        // Depth-first: resolve each declared sub-rule's value for this rule's
        // `invoke`. A sub-rule's own frame is not surfaced — only the root rule's
        // inputs are charted — but it is resolved through the same session.
        let mut child_values: HashMap<String, f64> = HashMap::new();
        for sub_id in &rule.subrules {
            let sub = registry.resolve(sub_id.as_str())?;
            let (sub_decision, _) = run_dry(session, registry, sub).await?;
            child_values.insert(sub.id.as_str().to_owned(), sub_decision.value);
        }

        // Resolve this rule's own window inputs, capturing the buckets so a caller
        // can chart the exact frame the rule decided on.
        let mut inputs: HashMap<String, f64> = HashMap::new();
        let mut resolved: Vec<ResolvedInput> = Vec::with_capacity(rule.inputs.len());
        for binding in &rule.inputs {
            let resolved_input = resolve_input(session, binding).await?;
            inputs.insert(binding.name.clone(), resolved_input.value);
            resolved.push(resolved_input);
        }

        let decision = run_script(rule, &inputs, child_values)?;
        Ok((decision, resolved))
    })
}

/// Resolve one binding to its window buckets and selected value.
///
/// Pulls the rollup through the scoped session (same scan
/// [`resolve`](crate::rule::resolve) uses) and selects the binding's aggregate
/// from the most recent bucket. An empty series is a [`RuleError::Binding`], not a
/// silent zero — a rule never dry-runs on a value that was never observed.
async fn resolve_input(session: &Surreal<Db>, binding: &Binding) -> Result<ResolvedInput> {
    let filter = binding
        .filter
        .as_ref()
        .map(|(key, value)| SeriesFilter { key, value });
    let buckets =
        rollup_window_filtered(session, binding.table, &binding.field, binding.grain, filter)
            .await
            .map_err(|e| RuleError::Window(e.to_string()))?;
    let latest = buckets.last().ok_or_else(|| {
        RuleError::Binding(format!(
            "no window bucket for '{}' from content.{}",
            binding.name, binding.field
        ))
    })?;
    let value = binding.aggregate.select(latest);
    Ok(ResolvedInput {
        name: binding.name.clone(),
        buckets,
        value,
    })
}
