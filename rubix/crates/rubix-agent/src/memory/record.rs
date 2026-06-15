//! The agent-memory record shape — taxonomy borrowed, store native.
//!
//! Memory is SurrealDB-native: there is no second store for vectors (SCOPE
//! principle 6; AGENT.md "Memory schema — borrow the taxonomy, import nothing").
//! A memory is a generic record whose free-form content carries the memory
//! *kind*, the text, and its L2-normalized embedding. The taxonomy
//! (working / semantic / episodic / procedural / preference / shared) is a
//! **schema reference** from SurrealDB's agent-memory demo and the Spectron model
//! — design input only, no dependency. The embedding lands at `content.embedding`,
//! the dotted path the scoped [`nearest`](rubix_query::nearest) search reads.

use serde::{Deserialize, Serialize};

/// The kind of memory, borrowed from the standard agent-memory taxonomy.
///
/// The kind is descriptive content, not a trust boundary — every kind is written
/// through the same gated `agent-memory-write` path and read on the same scoped
/// session. It is stored as a lowercase string so recall and dashboards can
/// filter on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    /// Short-lived scratch context for the current task.
    Working,
    /// Durable facts the agent has learned.
    Semantic,
    /// A record of something that happened (an event the agent observed/did).
    Episodic,
    /// How-to knowledge: a procedure the agent can replay.
    Procedural,
    /// A stated preference (the user's or the agent's operating preference).
    Preference,
    /// Memory shared across agents/principals in the namespace.
    Shared,
}

/// One agent memory: its kind, text, and normalized embedding.
///
/// Constructed from a caller-supplied embedding via [`MemoryRecord::new`], which
/// normalizes the vector so the stored geometry matches the recall probe's. The
/// embedding is private so it cannot be replaced with an un-normalized vector
/// after construction; [`MemoryRecord::content`] projects the record into the
/// free-form JSON the gate command persists.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryRecord {
    kind: MemoryKind,
    text: String,
    embedding: Vec<f64>,
}

impl MemoryRecord {
    /// Build a memory of `kind` holding `text`, with `embedding` normalized.
    ///
    /// The embedding is L2-normalized on construction
    /// ([`normalize_embedding`](crate::normalize_embedding)) so euclidean recall
    /// ranks it the way cosine would; the un-normalized vector is never stored.
    ///
    /// # Errors
    /// Returns [`AgentError::Embedding`](crate::AgentError::Embedding) if
    /// `embedding` is empty or has no usable magnitude.
    pub fn new(
        kind: MemoryKind,
        text: impl Into<String>,
        embedding: &[f64],
    ) -> crate::Result<Self> {
        let embedding = crate::normalize_embedding(embedding)?;
        Ok(Self {
            kind,
            text: text.into(),
            embedding,
        })
    }

    /// The memory's kind.
    #[must_use]
    pub fn kind(&self) -> MemoryKind {
        self.kind
    }

    /// The memory's text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The stored, already-normalized embedding.
    #[must_use]
    pub fn embedding(&self) -> &[f64] {
        &self.embedding
    }

    /// Project the memory into the free-form JSON content a gate command persists.
    ///
    /// The embedding is placed at `embedding` (top level), matching the dotted
    /// field path recall passes to [`nearest`](rubix_query::nearest). The kind and
    /// text sit beside it as plain content — the platform bakes in no fixed
    /// ontology (SCOPE principle 4), so structure comes from this content shape.
    #[must_use]
    pub fn content(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": self.kind,
            "text": self.text,
            "embedding": self.embedding,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryKind, MemoryRecord};

    #[test]
    fn a_memory_normalizes_its_embedding_on_construction() {
        let memory = MemoryRecord::new(MemoryKind::Semantic, "the chiller runs hot", &[3.0, 4.0])
            .expect("memory");
        let norm = memory
            .embedding()
            .iter()
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt();
        assert!((norm - 1.0).abs() < 1e-12);
    }

    #[test]
    fn content_carries_kind_text_and_embedding() {
        let memory =
            MemoryRecord::new(MemoryKind::Episodic, "pre-cooled L4 west", &[1.0]).expect("memory");
        let content = memory.content();
        assert_eq!(content["kind"], "episodic");
        assert_eq!(content["text"], "pre-cooled L4 west");
        assert_eq!(content["embedding"][0], 1.0);
    }

    #[test]
    fn an_unusable_embedding_is_refused() {
        assert!(MemoryRecord::new(MemoryKind::Working, "x", &[]).is_err());
        assert!(MemoryRecord::new(MemoryKind::Working, "x", &[0.0, 0.0]).is_err());
    }
}
