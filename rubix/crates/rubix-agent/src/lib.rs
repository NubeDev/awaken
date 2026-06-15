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
//! The LLM brain (provider loop, tool dispatch) is intentionally not wired here
//! yet: its adapter layer (Rig) sits above these seams, which exist so the brain
//! can never reach a plane the principal was not granted.

mod error;
mod memory;
mod provision;

pub use error::{AgentError, Result};
pub use memory::{
    MemoryKind, MemoryRecord, Persisted, Recalled, normalize_embedding, persist_memory,
    recall_memory,
};
pub use provision::{AgentPrincipal, AgentTier, provision_agent};
