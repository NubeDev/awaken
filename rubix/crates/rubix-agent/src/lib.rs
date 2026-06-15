//! AI agent runtime on the rubix substrate.
//!
//! This crate bolts an AI agent onto rubix as a **scoped principal**
//! (`rubix/docs/design/AGENT.md`), reusing the gate, capability grants,
//! scoped-session reads, and vector store rather than importing a framework that
//! brings its own. The agent is provisioned as an `Extension`-kind service account
//! ([`provision_agent`]), so its *commands* are authorized, audited, and
//! correlated by the gate, and its *reads* are scoped by SurrealDB row-level
//! permissions — the same two enforcement points every other principal crosses
//! (SCOPE "Two authz layers"; STACK contracts #1/#2).
//!
//! Two seams are genuinely rubix-specific because both must honor the gate:
//!
//! - [`provision`] provisions the agent principal and grants its [`AgentTier`]
//!   (analyst ⊂ operator ⊂ actuator), resolved onto the WS-04 grant set.
//! - [`memory`] is the agent's memory seam: recall runs as a nearest-neighbour
//!   search on the **scoped session** (a read, not a capability), and persistence
//!   crosses the gate as an `agent-memory-write` [`Command`](rubix_gate::Command)
//!   (a mutation). Embeddings are L2-normalized on write so SurrealDB's euclidean
//!   distance ranks identically to cosine (AGENT.md, open question 3c).
//!
//! The LLM brain (Rig) is wired behind the `cloud` feature ([`brain`]): it
//! constructs Rig's OpenAI client and exposes [`Brain::embed`] (the input to the
//! memory seam) and [`Brain::answer`] (a Rig agent turn). It is *provider wiring
//! only* — it never touches SurrealDB, the gate, or a scoped session, so it sits
//! strictly above the two seams and cannot reach a plane the principal was not
//! granted. The edge build (feature off) carries no cloud provider and fails
//! closed (AGENT.md, open question 1). Tool dispatch over the capability bridge is
//! the remaining `Tool`-seam follow-on.

#[cfg(feature = "cloud")]
mod brain;
mod error;
mod memory;
mod provision;

#[cfg(feature = "cloud")]
pub use brain::{
    Brain, BrainConfig, DEFAULT_COMPLETION_MODEL, DEFAULT_EMBEDDING_MODEL,
};
pub use error::{AgentError, Result};
pub use memory::{
    MemoryKind, MemoryRecord, Persisted, Recalled, normalize_embedding, persist_memory,
    recall_memory,
};
pub use provision::{AgentPrincipal, AgentTier, provision_agent};
