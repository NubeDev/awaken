//! Record a rule's decision as an insight through the WS-05 command gate.
//!
//! The decision is recorded back to SurrealDB through the gate
//! (`rubix/docs/sessions/WS-11.md`; `rubix/docs/SCOPE.md`, "Rhai — rules and
//! insights"), never written directly: the gate checks the principal's
//! `rule-invoke` capability, mints/carries the correlation id, captures
//! before/after atomically, and appends the immutable audit row (contracts #1,
//! #3, #4). An insight is an append-only data-plane firing
//! (`rubix-gate`'s undo classification), so it is created as a fresh generic
//! record — the platform bakes in no fixed ontology, structure comes from the
//! decision content (`rubix/docs/SCOPE.md`, principle 4). The carried correlation
//! id is the same thread stamped into the published event and the span tree.
//!
//! The recorded content also carries the §5c evaluation shape
//! (`rubix/docs/design/LAMINAR-BORROW.md`): the decision's `scores` map and a
//! `group_id` (falling back to the rule's identity), so a rule firing is a
//! comparable, chartable *evaluation datapoint* — audited and correlated to the
//! trace id like every other gate write. The same shape later covers agent-run
//! QA when the Rig brain lands.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{CorrelationId, Id, Principal};
use rubix_gate::{Capability, Change, Command, apply};

use crate::engine::Decision;
use crate::error::{Result, RuleError};
use crate::rule::Rule;

/// The capability a principal must hold to record a rule's insight.
///
/// Evaluating a rule is the app-enforced `rule-invoke` action (`rubix/docs/
/// SCOPE.md`, "Two authz layers"); the gate refuses the insight write if the
/// principal lacks the grant, fail closed before anything is persisted.
const RULE_CAPABILITY: Capability = Capability::RuleInvoke;

/// The record id a recorded insight is written under, and the gate result.
///
/// `insight_id` is the fresh generic-record id the decision was created at;
/// `correlation_id` is the id the gate carried onto the insight and its audit
/// row — the same thread the published event and span tree carry.
#[derive(Debug, Clone)]
pub struct Recorded {
    /// The id the insight record was created under.
    pub insight_id: Id,
    /// The correlation id the gate carried onto the insight.
    pub correlation_id: CorrelationId,
}

/// Record `decision` for `rule` as `principal`'s insight through the gate,
/// carrying `correlation`.
///
/// Builds a [`Command`] that creates a fresh insight record holding the decision
/// content and drives it through [`apply`], so the write is authorized, captured,
/// correlated, and audited in one path. The command runs as `principal` — the
/// principal evaluation was requested for — so the insight lands in its tenant
/// and is attributed to it; the namespace is the principal's own (the gate has no
/// cross-tenant write path). `db` is the gate's owner handle (the only session
/// past the audit table's append permission). Returns the insight id and the
/// carried correlation id.
///
/// # Errors
/// Returns [`RuleError::Record`] if the gate denies the command (the principal
/// lacks the `rule-invoke` grant) or the write fails.
pub async fn record_insight(
    db: &Surreal<Db>,
    principal: &Principal,
    rule: &Rule,
    decision: &Decision,
    correlation: CorrelationId,
) -> Result<Recorded> {
    let insight_id = Id::new();
    let command = Command::new(
        principal.clone(),
        RULE_CAPABILITY,
        insight_id.clone(),
        Change::Create(decision.to_content(&rule.output, rule.id.as_str())),
    );
    let applied = apply(db, &command, Some(correlation))
        .await
        .map_err(|e| RuleError::Record(e.to_string()))?;
    Ok(Recorded {
        insight_id,
        correlation_id: applied.correlation_id,
    })
}
