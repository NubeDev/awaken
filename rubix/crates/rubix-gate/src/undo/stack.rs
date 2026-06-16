//! The per-principal + per-resource undo/redo stack.
//!
//! `rubix/docs/SCOPE.md`, "Undo/redo": the undo stack is scoped **per principal +
//! resource** and is the *mutable* consumer of the gate's captured change (pushed
//! on a definition mutation, popped on undo; a redo stack is fed by undo). This
//! is the short-lived, session-scoped counterpart to the immutable audit log —
//! it is popped, the audit log never is. Entries carry everything needed to
//! re-enter the WS-05 gate (principal, capability, target, the reversible
//! [`ChangeRecord`], and the correlation id that threads the chain), so a reversal
//! is itself capability-checked and audited.

use std::collections::HashMap;

use rubix_core::{CorrelationId, Id, Principal};

use crate::capability::Capability;

use super::change::ChangeRecord;

/// One reversible step on the undo (or redo) stack.
///
/// Holds the reversible [`ChangeRecord`] plus the context the gate needs to
/// replay it: the acting principal, the capability the replay is gated by, the
/// target record, and the correlation id the original action ran under (carried
/// through the whole chain so audit/trace stay threaded).
#[derive(Debug, Clone, PartialEq)]
pub struct UndoEntry {
    /// The principal whose stack this entry belongs to.
    pub principal: Principal,
    /// The capability a replay of this change is gated by.
    pub capability: Capability,
    /// The record the change targets.
    pub target: Id,
    /// The forward/inverse pair.
    pub change: ChangeRecord,
    /// The correlation id threading this change to its audit and trace.
    pub correlation_id: CorrelationId,
}

impl UndoEntry {
    /// The stack key: scope is per principal subject + namespace + resource.
    fn key(&self) -> StackKey {
        StackKey {
            subject: self.principal.subject.to_string(),
            namespace: self.principal.namespace.clone(),
            target: self.target.to_string(),
        }
    }
}

/// The composite key the stack partitions entries by.
///
/// Scope is the principal (subject within its namespace) and the resource it
/// acted on, matching the SCOPE "per principal + resource" rule — two principals,
/// or one principal editing two resources, keep independent undo histories.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StackKey {
    subject: String,
    namespace: String,
    target: String,
}

/// An in-memory, session-scoped undo/redo store, partitioned per principal +
/// resource.
///
/// Each `(principal, resource)` slot has its own undo and redo stack. A
/// definition mutation pushes onto undo; an undo pops undo and pushes redo; a
/// redo pops redo and pushes undo — the classic linear model, scoped per
/// resource (SCOPE open question 9 notes collaborative models as future work;
/// this is the per-user linear baseline).
#[derive(Debug, Default)]
pub struct UndoStore {
    undo: HashMap<StackKey, Vec<UndoEntry>>,
    redo: HashMap<StackKey, Vec<UndoEntry>>,
}

impl UndoStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a freshly captured definition change onto its undo stack.
    ///
    /// A new forward action invalidates any pending redo history for that
    /// resource (the classic linear-undo rule), so the redo stack for the slot
    /// is cleared.
    pub fn push_undo(&mut self, entry: UndoEntry) {
        let key = entry.key();
        self.redo.remove(&key);
        self.undo.entry(key).or_default().push(entry);
    }

    /// Pop the most recent undoable change for a principal + resource.
    ///
    /// Returns `None` when nothing is left to undo for that slot.
    pub fn pop_undo(&mut self, principal: &Principal, target: &Id) -> Option<UndoEntry> {
        let key = key_for(principal, target);
        self.undo.get_mut(&key).and_then(Vec::pop)
    }

    /// Record an applied undo as a redo candidate for its slot.
    pub fn push_redo(&mut self, entry: UndoEntry) {
        let key = entry.key();
        self.redo.entry(key).or_default().push(entry);
    }

    /// Pop the most recent redoable change for a principal + resource.
    ///
    /// Returns `None` when nothing is left to redo for that slot.
    pub fn pop_redo(&mut self, principal: &Principal, target: &Id) -> Option<UndoEntry> {
        let key = key_for(principal, target);
        self.redo.get_mut(&key).and_then(Vec::pop)
    }

    /// Re-arm an undone change for undo after it was redone.
    pub fn rearm_undo(&mut self, entry: UndoEntry) {
        let key = entry.key();
        self.undo.entry(key).or_default().push(entry);
    }
}

/// Build the stack key for a principal acting on a resource.
fn key_for(principal: &Principal, target: &Id) -> StackKey {
    StackKey {
        subject: principal.subject.to_string(),
        namespace: principal.namespace.clone(),
        target: target.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use rubix_core::{CorrelationId, Id, Principal, PrincipalKind, Role};

    use crate::capability::Capability;
    use crate::command::Change;

    use super::super::change::ChangeRecord;
    use super::{UndoEntry, UndoStore};

    fn principal(subject: &str) -> Principal {
        Principal::new(
            Id::from_raw(subject),
            "tenant-a",
            PrincipalKind::User,
            Role::Operator,
        )
    }

    fn entry(subject: &str, target: &str) -> UndoEntry {
        UndoEntry {
            principal: principal(subject),
            capability: Capability::RuleInvoke,
            target: Id::from_raw(target),
            change: ChangeRecord {
                forward: Change::Update(serde_json::json!({ "v": 2 })),
                inverse: Change::Update(serde_json::json!({ "v": 1 })),
            },
            correlation_id: CorrelationId::carry("corr-1"),
        }
    }

    #[test]
    fn undo_pops_last_in_first_out_per_resource() {
        let mut store = UndoStore::new();
        store.push_undo(entry("alice", "rec-1"));
        store.push_undo(entry("alice", "rec-1"));
        assert!(
            store
                .pop_undo(&principal("alice"), &Id::from_raw("rec-1"))
                .is_some()
        );
        assert!(
            store
                .pop_undo(&principal("alice"), &Id::from_raw("rec-1"))
                .is_some()
        );
        assert!(
            store
                .pop_undo(&principal("alice"), &Id::from_raw("rec-1"))
                .is_none()
        );
    }

    #[test]
    fn stacks_are_isolated_per_principal_and_resource() {
        let mut store = UndoStore::new();
        store.push_undo(entry("alice", "rec-1"));
        assert!(
            store
                .pop_undo(&principal("bob"), &Id::from_raw("rec-1"))
                .is_none()
        );
        assert!(
            store
                .pop_undo(&principal("alice"), &Id::from_raw("rec-2"))
                .is_none()
        );
        assert!(
            store
                .pop_undo(&principal("alice"), &Id::from_raw("rec-1"))
                .is_some()
        );
    }

    #[test]
    fn a_new_forward_change_clears_pending_redo() {
        let mut store = UndoStore::new();
        store.push_redo(entry("alice", "rec-1"));
        store.push_undo(entry("alice", "rec-1"));
        assert!(
            store
                .pop_redo(&principal("alice"), &Id::from_raw("rec-1"))
                .is_none()
        );
    }
}
