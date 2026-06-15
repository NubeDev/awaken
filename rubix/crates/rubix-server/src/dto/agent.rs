//! Wire shapes for the agent surface (`rubix/docs/design/AGENT.md`).
//!
//! The agent is a scoped service-account principal: provisioning it is an admin
//! action that mirrors the principal surface (a subject, a generated-or-supplied
//! secret returned once, a tier), and its memory/ask surfaces map to the
//! `rubix-agent` seams — recall on the scoped session (a read), persist through
//! the gate (`agent-memory-write`), and a brain turn over the chosen provider.
//! Like every DTO these are kept separate from the domain types so the wire never
//! leaks a secret or a storage-prefixed subject.

use rubix_agent::{AgentTier, MemoryKind, Recalled};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The body of a provision-agent request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ProvisionAgentRequest {
    /// The API-local subject for the agent (stored namespace-prefixed, like any
    /// principal).
    pub subject: String,
    /// The tier to provision at: `analyst`, `operator`, or `actuator`. Strictly
    /// layered — analyst ⊂ operator ⊂ actuator (AGENT.md, "Analyst vs. operator").
    pub tier: String,
    /// The secret the agent authenticates with. Optional: when omitted the server
    /// generates one and returns it once (the only time a secret crosses the wire,
    /// mirroring the principal surface).
    #[serde(default)]
    pub secret: Option<String>,
}

/// A provisioned agent as returned to a client — identity + tier, no secret.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AgentDto {
    /// The API-local subject (the `{namespace}_` prefix is stripped).
    pub subject: String,
    /// The namespace (tenant) the agent belongs to.
    pub namespace: String,
    /// The tier the agent was provisioned at.
    pub tier: String,
}

/// The response to a provision request: the agent plus its minted secret.
///
/// The secret is returned **only** here and **only** when the server generated it
/// (a caller-supplied secret is echoed as `None`) — the single response that ever
/// carries an agent secret.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProvisionedAgentDto {
    /// The API-local subject.
    pub subject: String,
    /// The namespace (tenant) the agent belongs to.
    pub namespace: String,
    /// The tier the agent was provisioned at.
    pub tier: String,
    /// The generated secret, present only when the server minted it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

/// The body of a recall request: a probe embedding and how many neighbours.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecallRequest {
    /// The probe embedding to search near. Normalized by the seam before search,
    /// so a raw provider embedding is fine.
    pub probe: Vec<f64>,
    /// How many nearest memories to return.
    pub k: usize,
}

/// One recalled memory on the wire: its record id and distance from the probe.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecalledDto {
    /// The recalled memory record's id.
    pub id: String,
    /// Euclidean distance over normalized vectors (smaller is nearer) — orders
    /// identically to cosine similarity.
    pub distance: f64,
}

impl From<Recalled> for RecalledDto {
    fn from(hit: Recalled) -> Self {
        Self {
            id: hit.id,
            distance: hit.distance,
        }
    }
}

/// The body of a persist-memory request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PersistRequest {
    /// The memory kind: `working`, `semantic`, `episodic`, `procedural`,
    /// `preference`, or `shared`.
    pub kind: String,
    /// The memory text.
    pub text: String,
    /// The embedding; normalized by the seam on write so euclidean recall ranks it
    /// the way cosine would.
    pub embedding: Vec<f64>,
}

/// The response to a persist request: the memory id and the carried correlation id.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PersistedDto {
    /// The id the memory record was created under.
    pub memory_id: String,
    /// The correlation id the gate carried onto the memory and its audit row.
    pub correlation_id: String,
}

/// The body of an ask request: a free-form question and optional grounding.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AskRequest {
    /// The user's question.
    pub question: String,
    /// Optional grounding context the caller assembled on the scoped session
    /// (recalled memory, the site's records). Injected into the agent's preamble
    /// so the answer is conditioned on what the principal may see.
    #[serde(default)]
    pub context: Option<String>,
}

/// The response to an ask request.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AskResponse {
    /// The agent's answer.
    pub answer: String,
    /// Whether the answer came from the cloud brain (`true`) or the grounded,
    /// model-free fallback (`false`, when the `cloud` feature is off or no key is
    /// configured). Lets the UI label a degraded answer honestly.
    pub grounded: bool,
}

/// Parse a wire tier string into an [`AgentTier`], or `None` if unknown.
#[must_use]
pub fn parse_tier(raw: &str) -> Option<AgentTier> {
    match raw {
        "analyst" => Some(AgentTier::Analyst),
        "operator" => Some(AgentTier::Operator),
        "actuator" => Some(AgentTier::Actuator),
        _ => None,
    }
}

/// The stable wire string for a tier.
#[must_use]
pub fn tier_str(tier: AgentTier) -> &'static str {
    match tier {
        AgentTier::Analyst => "analyst",
        AgentTier::Operator => "operator",
        AgentTier::Actuator => "actuator",
    }
}

/// Parse a wire memory-kind string into a [`MemoryKind`], or `None` if unknown.
#[must_use]
pub fn parse_kind(raw: &str) -> Option<MemoryKind> {
    match raw {
        "working" => Some(MemoryKind::Working),
        "semantic" => Some(MemoryKind::Semantic),
        "episodic" => Some(MemoryKind::Episodic),
        "procedural" => Some(MemoryKind::Procedural),
        "preference" => Some(MemoryKind::Preference),
        "shared" => Some(MemoryKind::Shared),
        _ => None,
    }
}
