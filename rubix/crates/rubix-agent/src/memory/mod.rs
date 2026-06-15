//! The agent's memory seam — recall on the scoped session, persist through the gate.
//!
//! This is the load-bearing seam of the design (AGENT.md, "Memory seam over the
//! gate"): it keeps recall on the gate's **read** path (the scoped session + row
//! perms) and persistence on its **write** path (an `agent-memory-write`
//! command). Implementing it directly over `rubix-query` + the gate — rather than
//! adopting `rig-surrealdb`, which opens its own connection — is what stops memory
//! reads escaping row-perm scoping and memory writes escaping audit/correlation
//! /undo. Embeddings are L2-normalized so the euclidean-only search ranks
//! identically to cosine (open question 3c).

mod normalize;
mod persist;
mod recall;
mod record;

pub use normalize::normalize_embedding;
pub use persist::{Persisted, persist_memory};
pub use recall::{Recalled, recall_memory};
pub use record::{MemoryKind, MemoryRecord};
