//! Stable identifier for records and principals across the platform.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A stable, opaque identifier.
///
/// Backed by a UUID v4 so ids are globally unique without coordination — a
/// requirement for the edge-partitioned, append-only data plane where two edges
/// must never mint the same id (`rubix/docs/SCOPE.md`, "Append-only data,
/// edge-partitioned").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(String);

impl Id {
    /// Mint a fresh, unique id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Adopt an externally supplied id string (e.g. decoded from the store).
    #[must_use]
    pub fn from_raw(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// The id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Id;

    #[test]
    fn new_ids_are_unique() {
        let a = Id::new();
        let b = Id::new();
        assert_ne!(a, b);
    }

    #[test]
    fn from_raw_round_trips_through_as_str() {
        let id = Id::from_raw("record-42");
        assert_eq!(id.as_str(), "record-42");
        assert_eq!(id.to_string(), "record-42");
    }
}
