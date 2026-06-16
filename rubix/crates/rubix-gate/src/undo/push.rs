//! Push a captured definition mutation onto the per-principal undo stack.
//!
//! `rubix/docs/SCOPE.md`, "Undo/redo": the undo stack is the mutable consumer of
//! the gate's captured change, built **only for definition mutations**. This verb
//! is where a freshly applied command becomes an undoable step: it refuses any
//! non-definition kind at the boundary ([`is_undoable`]), derives the reversible
//! [`ChangeRecord`] from the same capture audit used (one capture, two
//! consumers), and pushes the entry onto the principal + resource stack carrying
//! the correlation id so the whole chain stays threaded.

use rubix_core::Principal;

use crate::capability::Capability;
use crate::command::{Applied, Change};
use crate::error::{GateError, Result};

use super::change::ChangeRecord;
use super::classify::{RecordKind, is_undoable};
use super::stack::{UndoEntry, UndoStore};

/// Push the result of an applied definition command onto `store`.
///
/// `kind` is the caller-declared class of the target record; only a
/// [`RecordKind::Definition`] is admitted — a data-plane or audit kind is
/// refused here and nothing is pushed (the undo boundary, failing closed).
/// `forward` is the change the command applied; `applied` carries the capture and
/// correlation id `apply` returned. The pushed entry restores the prior value
/// when later undone.
///
/// # Errors
/// Returns [`GateError::UndoBoundary`] if `kind` is not a definition.
pub fn push_change(
    store: &mut UndoStore,
    principal: &Principal,
    capability: Capability,
    target: &rubix_core::Id,
    kind: RecordKind,
    forward: &Change,
    applied: &Applied,
) -> Result<()> {
    if !is_undoable(kind) {
        return Err(GateError::UndoBoundary(format!(
            "{} records are not undoable (target {})",
            kind.as_str(),
            target
        )));
    }
    let change = ChangeRecord::from_capture(forward, &applied.captured);
    store.push_undo(UndoEntry {
        principal: principal.clone(),
        capability,
        target: target.clone(),
        change,
        correlation_id: applied.correlation_id.clone(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use rubix_core::{CorrelationId, Id, Principal, PrincipalKind, Role};

    use crate::capability::Capability;
    use crate::command::{Applied, CapturedChange, Change};
    use crate::error::GateError;

    use super::super::classify::RecordKind;
    use super::super::stack::UndoStore;
    use super::push_change;

    fn principal() -> Principal {
        Principal::new(
            Id::from_raw("alice"),
            "tenant-a",
            PrincipalKind::User,
            Role::Operator,
        )
    }

    fn applied() -> Applied {
        Applied {
            captured: CapturedChange {
                before: Some(serde_json::json!({ "v": 1 })),
                after: Some(serde_json::json!({ "v": 2 })),
            },
            correlation_id: CorrelationId::carry("corr-1"),
        }
    }

    #[test]
    fn a_definition_change_is_pushed() {
        let mut store = UndoStore::new();
        let target = Id::from_raw("rec-1");
        push_change(
            &mut store,
            &principal(),
            Capability::RuleInvoke,
            &target,
            RecordKind::Definition,
            &Change::Update(serde_json::json!({ "v": 2 })),
            &applied(),
        )
        .expect("definition push");
        assert!(store.pop_undo(&principal(), &target).is_some());
    }

    #[test]
    fn a_data_plane_change_is_refused_and_pushes_nothing() {
        let mut store = UndoStore::new();
        let target = Id::from_raw("reading-1");
        let err = push_change(
            &mut store,
            &principal(),
            Capability::RuleInvoke,
            &target,
            RecordKind::DataPlane,
            &Change::Create(serde_json::json!({ "temp": 21 })),
            &applied(),
        )
        .expect_err("data-plane refused");
        assert!(matches!(err, GateError::UndoBoundary(_)));
        assert!(store.pop_undo(&principal(), &target).is_none());
    }

    #[test]
    fn an_audit_change_is_refused() {
        let mut store = UndoStore::new();
        let err = push_change(
            &mut store,
            &principal(),
            Capability::RuleInvoke,
            &Id::from_raw("audit-1"),
            RecordKind::Audit,
            &Change::Delete,
            &applied(),
        )
        .expect_err("audit refused");
        assert!(matches!(err, GateError::UndoBoundary(_)));
    }
}
