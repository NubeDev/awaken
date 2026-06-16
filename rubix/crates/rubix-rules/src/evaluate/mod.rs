//! Evaluate a rule end-to-end: window values → decision → insight → event →
//! spans.
//!
//! The full rule runtime path (`rubix/docs/sessions/WS-11.md`;
//! `rubix/docs/SCOPE.md`, "Rhai — rules and insights"): one evaluation mints a
//! correlation id, evaluates the rule and its sub-rules depth-first against
//! DataFusion window values ([`compose`]), records the resulting decision as an
//! insight through the WS-05 gate ([`record`]), publishes the firing on the WS-07
//! bus ([`publish`]), and emits a WS-08 span tree per evaluation ([`span`]) — the
//! deterministic "why did this fire". The one correlation id is threaded through
//! the decision, the insight, the event, and every span (contract #3,
//! `rubix/STACK-DEISGN.md`), so a reader pivots across all four.

mod clock;
mod compose;
mod context;
mod dryrun;
mod publish;
mod record;
mod run;
mod span;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_bus::ControlBus;
use rubix_core::{CorrelationId, Id, Principal};
use rubix_trace::SampleRate;

use crate::engine::Decision;
use crate::error::{Result, RuleError};
use crate::rule::RuleRegistry;

use compose::evaluate_rule;
use context::EvalContext;
use publish::publish_insight;
use record::record_insight;

pub use dryrun::{DryRun, ResolvedInput, dry_run};
pub use publish::INSIGHT_EVENT_TYPE;
pub use record::Recorded;
pub use span::RULE_SPAN_NAME;

/// The handles a rule evaluation runs against.
///
/// `gate_db` is the store owner handle the gate writes the insight and audit row
/// on (the only session past the audit table's append permission, `rubix-gate`);
/// `session` is the principal's gate-issued scoped session window values are read
/// through (contract #1); `trace_db` is the owner handle spans are appended to.
/// They are passed once and reused across the recursive evaluation.
pub struct RuleRuntime<'a> {
    /// The gate owner handle the insight + audit write run on.
    pub gate_db: &'a Surreal<Db>,
    /// The principal's scoped session window values are read through.
    pub session: &'a Surreal<Db>,
    /// The owner handle the per-evaluation spans are appended to.
    pub trace_db: &'a Surreal<Db>,
    /// The in-process bus spans and the insight firing are emitted on.
    pub bus: &'a ControlBus,
    /// The sampling rate applied to each persisted span.
    pub sample: SampleRate,
}

/// The outcome of one rule evaluation, threaded by a single correlation id.
///
/// Carries the root rule's [`Decision`], the recorded insight's id, the
/// correlation id every span/insight/event shares, and the subscriber reach of
/// the published firing. A caller reassembles the span tree with
/// `rubix_trace::assemble_trace(trace_db, &evaluation.correlation_id)`.
#[derive(Debug, Clone)]
pub struct Evaluation {
    /// The root rule's decision.
    pub decision: Decision,
    /// The id the insight record was created under.
    pub insight_id: Id,
    /// The correlation id threading the decision, insight, event, and spans.
    pub correlation_id: CorrelationId,
    /// How many subscribers the published firing reached.
    pub event_reach: usize,
}

/// Evaluate the rule `root_id` in `registry` for `principal`, end-to-end.
///
/// Mints one correlation id, evaluates the rule and its sub-rules against window
/// values (emitting and persisting a span per node), records the root decision as
/// an insight through the gate carrying that id, and publishes the firing on the
/// bus under the same id. Returns the [`Evaluation`].
///
/// The order is deliberate: spans are emitted during evaluation (so a live trace
/// view sees the flow), the insight is recorded only after the decision is final
/// (so a recorded insight always reflects a complete evaluation), and the event
/// is published last (so a subscriber that reacts to the firing can already read
/// the recorded insight).
///
/// # Errors
/// Returns a [`RuleError`] if the root rule is unknown, a binding or sub-rule
/// fails to resolve, the script fails, the gate denies or fails the insight
/// write, or a span fails to persist — fail closed, never a partial firing.
pub async fn evaluate(
    runtime: &RuleRuntime<'_>,
    registry: &RuleRegistry,
    principal: &Principal,
    root_id: &Id,
) -> Result<Evaluation> {
    let root = registry.resolve(root_id.as_str())?;
    let correlation = CorrelationId::mint();

    let ctx = EvalContext {
        principal,
        registry,
        session: runtime.session,
        trace_db: runtime.trace_db,
        bus: runtime.bus,
        sample: runtime.sample,
        correlation: &correlation,
    };

    let decision = evaluate_rule(&ctx, root, None).await?;

    let recorded = record_insight(
        runtime.gate_db,
        principal,
        root,
        &decision,
        correlation.clone(),
    )
    .await?;

    // The gate must thread the same correlation id; a divergence would break the
    // contract-#3 pivot, so it is asserted rather than silently trusted.
    if recorded.correlation_id != correlation {
        return Err(RuleError::Record(
            "gate did not carry the evaluation correlation id".to_owned(),
        ));
    }

    let event_reach = publish_insight(
        runtime.bus,
        &recorded.insight_id,
        &root.output,
        &decision,
        &correlation,
    );

    Ok(Evaluation {
        decision,
        insight_id: recorded.insight_id,
        correlation_id: correlation,
        event_reach,
    })
}
