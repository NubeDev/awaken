//! The immutable audit projection of an applied command.
//!
//! Contract #4 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Audit log"): a
//! mutating command produces one append-only, immutable audit row — who did
//! what, when, with the before/after summary and the correlation id that threads
//! it to the trace and undo planes. This is the *immutable* projection of the
//! one captured change (`rubix/docs/SCOPE.md`, "Audit and undo derive from the
//! same captured change"); the mutable undo stack projects the inverse in WS-06.

use rubix_core::{CorrelationId, Id, Principal};

use crate::command::CapturedChange;

/// One audited mutation: principal, namespace, action, target, before/after,
/// correlation id.
///
/// Built from the [`Command`](crate::Command)'s principal/target/action, the
/// [`CapturedChange`] taken atomically with the write, and the correlation id
/// minted at the gate. The audit row is never updated or deleted — immutability
/// is enforced by SurrealDB table permissions (see
/// [`define_audit_schema`](crate::define_audit_schema)).
#[derive(Debug, Clone, PartialEq)]
pub struct AuditRecord {
    /// The subject of the principal that performed the action.
    pub subject: String,
    /// The namespace (tenant) the action occurred in.
    pub namespace: String,
    /// The action verb (`create` / `update` / `delete`).
    pub action: String,
    /// The id of the record the action targeted.
    pub target: String,
    /// The target's content before the mutation, if it existed.
    pub before: Option<serde_json::Value>,
    /// The content the mutation wrote, if any.
    pub after: Option<serde_json::Value>,
    /// The correlation id threading this action to its trace and undo entries.
    pub correlation_id: CorrelationId,
}

impl AuditRecord {
    /// Project an applied command's outcome into its immutable audit record.
    ///
    /// `action` is the change verb; `capture` carries the before/after taken
    /// atomically with the write; `correlation_id` is the id minted at the gate.
    #[must_use]
    pub(crate) fn project(
        principal: &Principal,
        action: &str,
        target: &Id,
        capture: &CapturedChange,
        correlation_id: &CorrelationId,
    ) -> Self {
        Self {
            subject: principal.subject.to_string(),
            namespace: principal.namespace.clone(),
            action: action.to_owned(),
            target: target.to_string(),
            before: capture.before.clone(),
            after: capture.after.clone(),
            correlation_id: correlation_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::{CorrelationId, Id, Principal, PrincipalKind, Role};

    use crate::command::CapturedChange;

    use super::AuditRecord;

    #[test]
    fn projection_carries_principal_action_and_correlation_id() {
        let principal = Principal::new(
            Id::from_raw("alice"),
            "tenant-a",
            PrincipalKind::User,
            Role::Operator,
        );
        let capture = CapturedChange {
            before: None,
            after: Some(serde_json::json!({ "temp": 21 })),
        };
        let correlation_id = CorrelationId::carry("corr-9");
        let audit = AuditRecord::project(
            &principal,
            "create",
            &Id::from_raw("rec-1"),
            &capture,
            &correlation_id,
        );
        assert_eq!(audit.subject, "alice");
        assert_eq!(audit.namespace, "tenant-a");
        assert_eq!(audit.action, "create");
        assert_eq!(audit.target, "rec-1");
        assert_eq!(audit.correlation_id, correlation_id);
        assert_eq!(audit.after, Some(serde_json::json!({ "temp": 21 })));
    }
}
