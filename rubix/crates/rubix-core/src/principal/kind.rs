//! What sort of principal an identity is.
//!
//! Users and extensions authenticate and are authorized the same way — one
//! identity model, two kinds (`rubix/docs/SCOPE.md`, principle 5: everything is
//! a scoped principal; extensions are service accounts).

use serde::{Deserialize, Serialize};

/// The kind of principal behind an identity.
///
/// The distinction is descriptive, not a trust boundary: both kinds cross the
/// same gate and are scoped by the same SurrealDB row-level permissions. An
/// extension is a service account, not a privileged plugin path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalKind {
    /// A human user.
    User,
    /// An extension service account.
    Extension,
}

#[cfg(test)]
mod tests {
    use super::PrincipalKind;

    #[test]
    fn kind_serialises_lowercase() {
        let json = serde_json::to_string(&PrincipalKind::Extension).expect("serialise");
        assert_eq!(json, "\"extension\"");
        let kind: PrincipalKind = serde_json::from_str("\"user\"").expect("deserialise");
        assert_eq!(kind, PrincipalKind::User);
    }
}
