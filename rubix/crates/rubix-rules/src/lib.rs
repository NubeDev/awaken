//! Rhai rules / insights runtime for the rubix platform.
//!
//! The embedded, deterministic rule/insight runtime (`rubix/docs/SCOPE.md`,
//! "Rhai — rules and insights"; `rubix/STACK-DEISGN.md`, "Rhai owns the decision;
//! DataFusion owns the data"). Rules are composable (a rule invokes another
//! rule), fire offline with no cloud dependency, and consume the time-window math
//! computed by DataFusion (WS-09) as their inputs. One evaluation produces a
//! decision that is:
//!
//! - **recorded** back to SurrealDB through the WS-05 command gate (authorized,
//!   captured, correlated, audited — never a direct write);
//! - **published** as a data-change event on the WS-07 in-process bus;
//! - **traced** as a WS-08 span tree per evaluation — which sub-rules ran, the
//!   values they saw, and the decision — the deterministic "why did this fire".
//!
//! One correlation id threads the decision, the insight, the event, and every
//! span (contract #3, `rubix/STACK-DEISGN.md`). Heavy aggregation never lives in
//! Rhai: the window values arrive pre-computed from `rubix-query`.
//!
//! The crate is laid out one verb per file (`rubix/docs/FILE-LAYOUT.md`):
//! [`engine`] builds the Rhai engine and the [`Decision`] a script returns;
//! [`rule`] is the rule model (script + input bindings + composition registry);
//! [`evaluate`] orchestrates the end-to-end evaluation phases.

mod engine;
mod error;
mod evaluate;
mod rule;

pub use engine::Decision;
pub use error::{Result, RuleError};
pub use evaluate::{
    Evaluation, INSIGHT_EVENT_TYPE, RULE_SPAN_NAME, Recorded, RuleRuntime, evaluate,
};
pub use rule::{Aggregate, Binding, Rule, RuleRegistry};
