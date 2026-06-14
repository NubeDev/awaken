//! The mutation a command intends to apply to a record.
//!
//! A command crosses the gate carrying exactly one intended record change
//! (`rubix/docs/SCOPE.md`, "Commands go through the gate"). The platform has no
//! fixed ontology, so a change is create/update/delete over free-form JSON
//! content — the same generic record shape `rubix-core` owns. The atomic
//! before/after capture ([`capture`](super::super::command)) runs this change
//! with SurrealDB `RETURN BEFORE` so the prior value is taken with the write.

use serde::{Deserialize, Serialize};

/// The intended change a command applies to its target record.
///
/// Each variant maps to one SurrealQL mutation the capture step runs with
/// `RETURN BEFORE`. `Create` and `Update` carry the new content; `Delete`
/// carries none. The action's verb is the audited action string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Change {
    /// Create the target record with this content.
    Create(serde_json::Value),
    /// Replace the target record's content with this content.
    Update(serde_json::Value),
    /// Delete the target record.
    Delete,
}

impl Change {
    /// The stable action verb stamped onto the audit record.
    #[must_use]
    pub fn action(&self) -> &'static str {
        match self {
            Change::Create(_) => "create",
            Change::Update(_) => "update",
            Change::Delete => "delete",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Change;

    #[test]
    fn action_names_the_verb() {
        assert_eq!(Change::Create(serde_json::json!({})).action(), "create");
        assert_eq!(Change::Update(serde_json::json!({})).action(), "update");
        assert_eq!(Change::Delete.action(), "delete");
    }
}
