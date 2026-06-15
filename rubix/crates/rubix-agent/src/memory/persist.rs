//! Persist an agent memory through the gate (the `agent-memory-write` command).
//!
//! Storing a memory is a **mutation**, so it crosses the gate (contract #1) on the
//! same path insights already follow
//! ([record.rs](../../../rubix-rules/src/evaluate/record.rs)): the gate checks the
//! principal's `agent-memory-write` grant, mints/carries the correlation id,
//! captures before/after atomically, and appends the immutable audit row. There is
//! **no generic write-any-record path** — every gate command authorizes a named
//! capability first — and none of the five original capabilities fits agent
//! memory, which is why `agent-memory-write` is its own fail-closed grant
//! (AGENT.md, "Memory writes cross the gate"). The memory is created as a fresh
//! generic record; structure comes from its content (SCOPE principle 4).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::{CorrelationId, Id, Principal};
use rubix_gate::{Capability, Change, Command, apply};

use crate::error::{AgentError, Result};

use super::record::MemoryRecord;

/// The capability a principal must hold to persist agent memory.
///
/// Persisting memory is the app-enforced `agent-memory-write` action (AGENT.md);
/// the gate refuses the write if the principal lacks the grant, fail closed before
/// anything is persisted. A read-only analyst still holds this grant because
/// recording recall is a mutation even when the agent itself only reads.
const MEMORY_CAPABILITY: Capability = Capability::AgentMemoryWrite;

/// A persisted memory: the record id it was created under and the carried id.
///
/// `memory_id` is the fresh generic-record id the memory was created at;
/// `correlation_id` is the id the gate carried onto the memory and its audit row —
/// the same thread the trace and any downstream event carry.
#[derive(Debug, Clone, PartialEq)]
pub struct Persisted {
    /// The id the memory record was created under.
    pub memory_id: Id,
    /// The correlation id the gate carried onto the memory.
    pub correlation_id: CorrelationId,
}

/// Persist `memory` as `principal`'s agent memory through the gate, carrying
/// `correlation`.
///
/// Builds a [`Command`] that creates a fresh memory record holding the memory's
/// content (kind, text, normalized embedding) and drives it through [`apply`], so
/// the write is authorized against `agent-memory-write`, captured, correlated, and
/// audited in one path. The command runs as `principal`, so the memory lands in
/// its tenant and is attributed to it; the namespace is the principal's own (the
/// gate has no cross-tenant write path). `db` is the gate's owner handle. Returns
/// the memory id and the carried correlation id.
///
/// # Errors
/// Returns [`AgentError::MemoryWrite`] if the gate denies the command (the
/// principal lacks the `agent-memory-write` grant) or the write fails.
pub async fn persist_memory(
    db: &Surreal<Db>,
    principal: &Principal,
    memory: &MemoryRecord,
    correlation: Option<CorrelationId>,
) -> Result<Persisted> {
    let memory_id = Id::new();
    let command = Command::new(
        principal.clone(),
        MEMORY_CAPABILITY,
        memory_id.clone(),
        Change::Create(memory.content()),
    );
    let applied = apply(db, &command, correlation)
        .await
        .map_err(|e| AgentError::MemoryWrite(e.to_string()))?;
    Ok(Persisted {
        memory_id,
        correlation_id: applied.correlation_id,
    })
}
