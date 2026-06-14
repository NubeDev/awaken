//! Classify a record kind and decide whether a change to it is undoable.
//!
//! Boundary (`rubix/docs/SCOPE.md`, "Undo/redo"): undo covers **user-facing
//! definitions only** (dashboards, rules, tags, datasource config) — never the
//! data plane (readings, insight firings) and never the audit log. The gate has
//! no fixed ontology, so the kind a mutation targets is declared by the caller
//! as a [`RecordKind`]; this verb is the single decision point that admits a
//! definition change onto the undo stack and refuses everything else, failing
//! closed so a data-plane or audit record can never be captured for reversal.

/// The class of record a mutation targets, for undo-boundary enforcement.
///
/// The platform stores every record in one generic table (`rubix-core`'s
/// `record`), so the kind is not a table name but the *role* the record plays:
/// a reversible definition, an append-only data-plane record, or the immutable
/// audit log. Only [`RecordKind::Definition`] is undoable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordKind {
    /// A user-facing definition: dashboard, rule, tag, or datasource config.
    /// The only undoable kind.
    Definition,
    /// An append-only data-plane record: a reading or an insight firing. Never
    /// undoable — the data plane is edge-partitioned and append-only.
    DataPlane,
    /// The append-only, immutable audit log. Never undoable — audit is the
    /// truth the undo stack is reconciled against, not a target of it.
    Audit,
}

impl RecordKind {
    /// The stable wire/storage string for this kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RecordKind::Definition => "definition",
            RecordKind::DataPlane => "data-plane",
            RecordKind::Audit => "audit",
        }
    }
}

/// Whether a change to a record of `kind` may be pushed onto the undo stack.
///
/// Returns `true` only for [`RecordKind::Definition`]. The data plane and the
/// audit log are out of bounds by design, so this fails closed: any kind other
/// than a definition is refused.
#[must_use]
pub fn is_undoable(kind: RecordKind) -> bool {
    matches!(kind, RecordKind::Definition)
}

#[cfg(test)]
mod tests {
    use super::{RecordKind, is_undoable};

    #[test]
    fn only_a_definition_is_undoable() {
        assert!(is_undoable(RecordKind::Definition));
    }

    #[test]
    fn a_data_plane_record_is_refused() {
        assert!(!is_undoable(RecordKind::DataPlane));
    }

    #[test]
    fn an_audit_record_is_refused() {
        assert!(!is_undoable(RecordKind::Audit));
    }

    #[test]
    fn kinds_have_stable_strings() {
        assert_eq!(RecordKind::Definition.as_str(), "definition");
        assert_eq!(RecordKind::DataPlane.as_str(), "data-plane");
        assert_eq!(RecordKind::Audit.as_str(), "audit");
    }
}
