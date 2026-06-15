//! Recall agent memory on the scoped session (a read, not a capability).
//!
//! Recall is the agent's `VectorStoreIndex` read seam (AGENT.md, "Seam mapping"):
//! a nearest-neighbour search run on the gate-issued **scoped session**, so
//! SurrealDB row-level permissions decide which memories are candidates (contract
//! #1/#2). It is **not** a capability — no grant is checked, the session's row
//! perms are the whole fence. The probe is L2-normalized to match the geometry
//! memories were stored under ([normalize.rs](super::normalize)), so the
//! euclidean-only [`nearest`](rubix_query::nearest) search ranks results the way
//! cosine similarity would.

use rubix_gate::ScopedSession;
use rubix_query::nearest;

use crate::error::{AgentError, Result};

/// The table agent memories are stored in, and the dotted field path its
/// embedding lives at.
///
/// A memory is persisted through the gate, which writes every command's content
/// into the generic `record` table nested under a `content` field
/// ([persist.rs](../../../rubix-gate/src/command/persist.rs)). So the memory's
/// embedding — written at `embedding` inside the command content — lands at
/// `content.embedding` on the stored row, the dotted path the scoped
/// [`nearest`](rubix_query::nearest) search reads. Both identifiers are fixed (not
/// caller-supplied) so recall reads a known shape.
const MEMORY_TABLE: &str = "record";
const EMBEDDING_FIELD: &str = "content.embedding";

/// One recalled memory: the record id and its distance from the probe.
///
/// `distance` is the euclidean distance over normalized vectors (smaller is
/// nearer), which orders identically to cosine similarity. The id is the memory
/// record's id; the caller reads the memory's content on the same scoped session
/// if it needs the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Recalled {
    /// The recalled memory record's id.
    pub id: String,
    /// The euclidean distance from the probe over normalized vectors.
    pub distance: f64,
}

/// Recall the `k` memories nearest `probe` on `session`'s scoped connection.
///
/// Normalizes `probe` to the stored geometry, then runs the nearest-neighbour
/// search on the principal's scoped session so only memories it may read can
/// match. A `k` of zero yields no hits. The search reads the fixed `memory` table
/// and its `embedding` field.
///
/// # Errors
/// Returns [`AgentError::Embedding`] if `probe` cannot be normalized (empty or
/// zero magnitude), or [`AgentError::MemoryRecall`] if the search fails.
pub async fn recall_memory(
    session: &ScopedSession,
    probe: &[f64],
    k: usize,
) -> Result<Vec<Recalled>> {
    if k == 0 {
        return Ok(Vec::new());
    }
    let probe = crate::normalize_embedding(probe)?;
    let hits = nearest(session.connection(), MEMORY_TABLE, EMBEDDING_FIELD, &probe, k)
        .await
        .map_err(|e| AgentError::MemoryRecall(e.to_string()))?;
    Ok(hits
        .into_iter()
        .map(|hit| Recalled {
            id: hit.id,
            distance: hit.distance,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{EMBEDDING_FIELD, MEMORY_TABLE};

    #[test]
    fn recall_reads_the_gate_record_shape() {
        // The gate writes command content into the generic `record` table nested
        // under `content`, so the embedding lands at `content.embedding`. A drift
        // here would silently return no hits.
        assert_eq!(MEMORY_TABLE, "record");
        assert_eq!(EMBEDDING_FIELD, "content.embedding");
    }
}
