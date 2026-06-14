//! The engine's view of the BMS. reflow actors depend on this trait, not on
//! the server's store/bus directly — keeping `rubix-flow` free of axum, sqlite,
//! and zenoh. `rubix-server` implements it; tests provide a fake.
//!
//! Point writes always go through the priority array (never a raw set), per
//! STACK-DEISGN.md: "point write (always through the priority array, never
//! raw)".

use std::sync::Arc;

use async_trait::async_trait;
use rubix_core::{HisSample, PointValue, SparkSeverity};
use rubix_rules::RuleStore;

use crate::error::FlowAccessError;

/// A finding a rule board wants to record. The board names the owning site by
/// its `{org}/{site}` keyexpr prefix (the same way it addresses points); the
/// host resolves it to a site id when persisting. Carries no id/timestamp —
/// the host assigns those — so the node stays free of the store's identity
/// scheme.
#[derive(Debug, Clone, PartialEq)]
pub struct SparkDraft {
    /// `{org}/{site}` prefix identifying the owning site.
    pub site_prefix: String,
    /// Rule identity, the `{rule}` segment of `{org}/{site}/spark/{rule}/**`.
    pub rule: String,
    pub severity: SparkSeverity,
    pub message: String,
}

/// A request for the embedded agent, raised by an `agent_call` board node. The
/// board hands a free-text prompt (and a thread key to group related calls).
/// Two activation modes: detached (fire-and-forget, the control-board default —
/// the node acknowledges and the run proceeds out-of-band, like a dispatched
/// spark) or awaited (the node blocks on the run and surfaces its outcome on an
/// outport so downstream nodes can branch on the agent's decision).
#[derive(Debug, Clone, PartialEq)]
pub struct AgentRequest {
    /// Agent thread to run on; groups repeated calls from the same board.
    pub thread: String,
    /// The prompt handed to the agent.
    pub prompt: String,
}

/// The outcome of an awaited agent run, projected for a board graph. Carries the
/// agent's final response and run identity so a downstream node can branch on
/// the decision without depending on the awaken runtime types — keeping
/// `rubix-flow` free of the runtime crate, the same boundary as [`PointAccess`].
#[derive(Debug, Clone, PartialEq)]
pub struct AgentOutcome {
    /// Run identity of the completed agent run.
    pub run_id: String,
    /// The agent's final natural-language response.
    pub response: String,
    /// Loop steps the run took before terminating.
    pub steps: usize,
}

/// What a `datasource` board node asks an external datasource to run. Either
/// operator-authored native SQL (the widget/spark trust tier) or an
/// operator-registered named query invoked by name — both supplied in the board
/// definition, never end-user input (docs/design/datasources.md "Query
/// authoring tiers"). Kept as a flow-local intent so `rubix-flow` need not depend
/// on the `rubix-datasource` engine crate (the same boundary that keeps it free
/// of axum/sqlite/zenoh); the host maps it onto the executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasourceQuery {
    /// Operator-authored native SQL with `$1`-style placeholders.
    Sql(String),
    /// An operator-registered named query, invoked by name.
    Named(String),
}

/// Read/command/query access to points, addressed by zenoh keyexpr prefix
/// (`{org}/{site}/{equip-path}/{point}`) so graphs reference points the same
/// way the bus and tags do. Also the sink for rule-board findings (sparks) and
/// the entry point for `agent_call` — both part of the BMS the engine acts on,
/// not a separate transport.
#[async_trait]
pub trait PointAccess: Send + Sync + 'static {
    /// Current effective value of a point, or `None` if unset/unknown.
    async fn read_point(&self, keyexpr: &str) -> Result<Option<PointValue>, FlowAccessError>;

    /// Command a priority slot (1..=16). Returns the new effective value.
    async fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> Result<Option<PointValue>, FlowAccessError>;

    /// History samples for a point, most recent first, capped at `limit`.
    async fn query_his(&self, keyexpr: &str, limit: usize)
        -> Result<Vec<HisSample>, FlowAccessError>;

    /// Record a rule-board finding. The default implementation rejects it, so
    /// a `PointAccess` that does not back a store (test fakes) need not handle
    /// sparks; the server's store-backed impl overrides this.
    async fn emit_spark(&self, _draft: SparkDraft) -> Result<(), FlowAccessError> {
        Err(FlowAccessError::Unsupported(
            "emit_spark: this point access does not record sparks".into(),
        ))
    }

    /// Raise an agent request from an `agent_call` node. The default rejects it,
    /// so a `PointAccess` without an agent runtime (test fakes, and crucially
    /// the board access the agent's own `run_board` tool builds — which breaks
    /// the agent → board → agent recursion) need not handle it. The server's
    /// scheduler/HTTP-backed impl overrides this to activate a detached run.
    async fn request_agent(&self, _request: AgentRequest) -> Result<(), FlowAccessError> {
        Err(FlowAccessError::Unsupported(
            "agent_call: this point access has no agent runtime".into(),
        ))
    }

    /// Raise an agent request and await the run, returning its [`AgentOutcome`].
    /// Used by an `agent_call` node configured to await so the agent's decision
    /// flows downstream in the same board run. The default rejects it for the
    /// same fail-closed reason as [`Self::request_agent`].
    async fn request_agent_awaited(
        &self,
        _request: AgentRequest,
    ) -> Result<AgentOutcome, FlowAccessError> {
        Err(FlowAccessError::Unsupported(
            "agent_call: this point access has no agent runtime".into(),
        ))
    }

    /// The tenant-scoped store a `rule` node resolves stored rules and
    /// composition (`rule(name, …)`) through. The default returns `None`, so a
    /// `PointAccess` that does not back a rule store (test fakes, the agent's own
    /// board access) makes a stored-rule `rule` node fail closed. The server's
    /// store-backed impl overrides this. An inline-script `rule` node needs no
    /// store and runs regardless. A pure accessor (no I/O), so it stays sync.
    fn rule_store(&self) -> Option<Arc<dyn RuleStore>> {
        None
    }

    /// Run a read against an external datasource for a `datasource` board node
    /// and return the `{ columns, rows, breached }` blob as JSON (the same shape
    /// `query_his` emits, so a downstream `rule` node folds it identically).
    ///
    /// `params` is the JSON parameter array (`[{type,value}, …]`) bound
    /// positionally — never spliced into SQL. This is the *strict* (spark) path:
    /// a result that breaches the datasource's caps is an `Err`, not a truncated
    /// grid, because a spark folding partial rows into a finding can silently
    /// reach a wrong conclusion (docs/design/datasources.md "Truncation on the
    /// spark path"). The default rejects it, so a `PointAccess` with no
    /// datasource registry (test fakes, the agent's own board access) makes a
    /// `datasource` node fail closed; the server's registry-backed impl
    /// overrides it.
    async fn query_datasource(
        &self,
        _datasource: &str,
        _query: DatasourceQuery,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, FlowAccessError> {
        Err(FlowAccessError::Unsupported(
            "datasource: this point access has no datasource registry".into(),
        ))
    }
}
