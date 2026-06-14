//! The shared context one rule evaluation threads through its phases.
//!
//! Evaluating a rule pulls window values (the scoped query session), runs the
//! script, and emits/persists spans (the bus + trace store) — all under one
//! correlation id (`rubix/docs/sessions/WS-11.md`). Rather than pass that handle
//! set through every recursive [`evaluate_rule`](super::compose::evaluate_rule)
//! call, they are bundled here once. The context borrows its handles for the
//! duration of one evaluation; it owns nothing, so it imposes no lifetime on the
//! caller's stores beyond the call.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_bus::ControlBus;
use rubix_core::{CorrelationId, Principal};
use rubix_trace::SampleRate;

use crate::rule::RuleRegistry;

/// The borrowed handles and identity one evaluation runs against.
pub struct EvalContext<'a> {
    /// The principal evaluation runs for — the insight's tenant and audit subject.
    pub principal: &'a Principal,
    /// The rule set sub-rules are resolved from for composition.
    pub registry: &'a RuleRegistry,
    /// The gate-issued scoped session window values are read through (contract #1).
    pub session: &'a Surreal<Db>,
    /// The trace store the per-evaluation spans are appended to (owner handle).
    pub trace_db: &'a Surreal<Db>,
    /// The in-process bus spans and the insight firing are emitted on.
    pub bus: &'a ControlBus,
    /// The sampling rate applied to each persisted span.
    pub sample: SampleRate,
    /// The correlation id every span, the insight, and the event share.
    pub correlation: &'a CorrelationId,
}

impl EvalContext<'_> {
    /// The namespace spans are written under — the principal's own tenant.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.principal.namespace
    }
}
