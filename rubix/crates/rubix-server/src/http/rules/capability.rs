//! The capability a rule-definition mutation through the transport requires.
//!
//! Every rule write crosses the WS-05 gate, which checks an app-enforced
//! capability grant before applying (`rubix/docs/SCOPE.md`, "Two authz layers").
//! Authoring a rule is [`RuleDefine`](rubix_gate::Capability::RuleDefine) — the
//! grant the committed capability set carves out for *mutating a rule definition*,
//! deliberately distinct from [`RuleInvoke`](rubix_gate::Capability::RuleInvoke),
//! which only *evaluates* a rule and records its decision (`rubix-gate`,
//! `Capability::RuleDefine` docs). Creating, editing, and deleting a rule all gate
//! on this one capability, named here once so they cannot drift apart.

use rubix_gate::Capability;

/// The capability grant a rule-definition mutation requires.
pub(crate) const RULE_WRITE: Capability = Capability::RuleDefine;
