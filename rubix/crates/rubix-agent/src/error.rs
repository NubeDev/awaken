//! Agent-runtime errors, converted into the project error at the boundary.
//!
//! The agent is a scoped service-account principal on the rubix substrate
//! (`rubix/docs/design/AGENT.md`): its failures are provisioning failures (the
//! agent principal could not be created or granted its tier) and memory-seam
//! failures (recall on the scoped session, or a memory write that the gate
//! refused or could not persist). They convert into the project
//! [`Error`](rubix_core::Error) so callers chain with `.context()` (CLAUDE.md
//! "Key Patterns"). A refused memory write fails closed before anything is
//! persisted — never a silent allow.

use rubix_core::Error as CoreError;

/// Convenience alias for agent results.
pub type Result<T> = std::result::Result<T, AgentError>;

/// Failures raised by the agent runtime's substrate seams.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AgentError {
    /// Provisioning the agent's service-account principal failed (the identity
    /// write or one of its tier grants).
    #[error("failed to provision agent: {0}")]
    Provision(String),

    /// Persisting agent memory through the gate failed — either the gate denied
    /// the command (the principal lacks the `agent-memory-write` grant) or the
    /// write itself failed (fail closed, `rubix/docs/design/AGENT.md`, "Memory
    /// writes cross the gate").
    #[error("failed to persist agent memory: {0}")]
    MemoryWrite(String),

    /// Recalling agent memory on the scoped session failed (the
    /// nearest-neighbour search returned an error).
    #[error("failed to recall agent memory: {0}")]
    MemoryRecall(String),

    /// An embedding was rejected before it could be persisted — an empty vector
    /// carries no direction to normalize, so it cannot be stored as a recallable
    /// memory.
    #[error("invalid embedding: {0}")]
    Embedding(String),

    /// The cloud brain (Rig/OpenAI) could not be reached or returned an error:
    /// a missing/invalid API key, a provider transport failure, or a completion
    /// the provider refused. The brain is behind the `cloud` feature and fails
    /// closed when absent (AGENT.md, open question 1) — a degraded caller falls
    /// back to a grounded, model-free answer rather than fabricating one.
    #[error("agent brain failed: {0}")]
    Brain(String),
}

impl From<AgentError> for CoreError {
    fn from(err: AgentError) -> Self {
        CoreError::Store(err.to_string())
    }
}
