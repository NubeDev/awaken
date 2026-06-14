//! The coarse authority a principal carries within its namespace.
//!
//! The role is a coarse band; fine-grained authority is the capability-grant
//! layer (`rubix/docs/SCOPE.md`, "Two authz layers"), built in a later
//! workstream. Data-record reads are scoped by namespace, not by role — the
//! role narrows what the app-enforced gate permits.

use serde::{Deserialize, Serialize};

/// The authority band of a principal inside its namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read-only access to the principal's namespace data.
    Viewer,
    /// Read and command access within the principal's namespace.
    Operator,
    /// Full authority over the principal's namespace.
    Admin,
}

#[cfg(test)]
mod tests {
    use super::Role;

    #[test]
    fn role_serialises_lowercase() {
        let json = serde_json::to_string(&Role::Operator).expect("serialise");
        assert_eq!(json, "\"operator\"");
        let role: Role = serde_json::from_str("\"admin\"").expect("deserialise");
        assert_eq!(role, Role::Admin);
    }
}
