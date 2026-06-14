//! Correlation id — the linchpin that threads an action across audit, undo,
//! traces, and bus events.
//!
//! Minted at the gate for principal actions or at ingest for data, then carried
//! on every bus event and stamped into audit/undo/trace records
//! (`rubix/docs/SCOPE.md`, "Correlation id (the linchpin)"; contract #3 in
//! `rubix/STACK-DEISGN.md`).

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The single thread that lets a reader pivot from an insight to its rule-run
/// trace to the audit of any action it triggered.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Mint a fresh correlation id at a chokepoint (gate or ingest).
    #[must_use]
    pub fn mint() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Carry an existing correlation id (e.g. propagated across a boundary).
    #[must_use]
    pub fn carry(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// The correlation id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::CorrelationId;

    #[test]
    fn minted_ids_are_unique() {
        assert_ne!(CorrelationId::mint(), CorrelationId::mint());
    }

    #[test]
    fn carry_preserves_the_propagated_value() {
        let carried = CorrelationId::carry("corr-7");
        assert_eq!(carried.as_str(), "corr-7");
    }
}
