//! The engine's view of the BMS. reflow actors depend on this trait, not on
//! the server's store/bus directly — keeping `rubix-flow` free of axum, sqlite,
//! and zenoh. `rubix-server` implements it; tests provide a fake.
//!
//! Point writes always go through the priority array (never a raw set), per
//! STACK-DEISGN.md: "point write (always through the priority array, never
//! raw)".

use rubix_core::{HisSample, PointValue, SparkSeverity};

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
/// board hands a free-text prompt (and a thread key to group related calls);
/// the host activates an agent run out-of-band. Fire-and-forget by design — a
/// control board must not block on an LLM round-trip, so the node acknowledges
/// the request and the run proceeds detached, exactly like a dispatched spark.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentRequest {
    /// Agent thread to run on; groups repeated calls from the same board.
    pub thread: String,
    /// The prompt handed to the agent.
    pub prompt: String,
}

/// Read/command/query access to points, addressed by zenoh keyexpr prefix
/// (`{org}/{site}/{equip-path}/{point}`) so graphs reference points the same
/// way the bus and tags do. Also the sink for rule-board findings (sparks) and
/// the entry point for `agent_call` — both part of the BMS the engine acts on,
/// not a separate transport.
pub trait PointAccess: Send + Sync + 'static {
    /// Current effective value of a point, or `None` if unset/unknown.
    fn read_point(&self, keyexpr: &str) -> anyhow::Result<Option<PointValue>>;

    /// Command a priority slot (1..=16). Returns the new effective value.
    fn write_point(
        &self,
        keyexpr: &str,
        priority: u8,
        value: PointValue,
    ) -> anyhow::Result<Option<PointValue>>;

    /// History samples for a point, most recent first, capped at `limit`.
    fn query_his(&self, keyexpr: &str, limit: usize) -> anyhow::Result<Vec<HisSample>>;

    /// Record a rule-board finding. The default implementation rejects it, so
    /// a `PointAccess` that does not back a store (test fakes) need not handle
    /// sparks; the server's store-backed impl overrides this.
    fn emit_spark(&self, _draft: SparkDraft) -> anyhow::Result<()> {
        anyhow::bail!("emit_spark: this point access does not record sparks")
    }

    /// Raise an agent request from an `agent_call` node. The default rejects it,
    /// so a `PointAccess` without an agent runtime (test fakes, and crucially
    /// the board access the agent's own `run_board` tool builds — which breaks
    /// the agent → board → agent recursion) need not handle it. The server's
    /// scheduler/HTTP-backed impl overrides this to activate a detached run.
    fn request_agent(&self, _request: AgentRequest) -> anyhow::Result<()> {
        anyhow::bail!("agent_call: this point access has no agent runtime")
    }
}
