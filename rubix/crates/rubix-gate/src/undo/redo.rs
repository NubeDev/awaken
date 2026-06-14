//! Redo a previously undone definition mutation through the gate.
//!
//! `rubix/docs/SCOPE.md`, "Undo/redo": a redo stack is fed by undo. This verb
//! pops the most recent entry the [`undo`](super::undo) verb moved onto the redo
//! stack and re-applies its **forward** change through the WS-05 gate — so, like
//! undo, the redo is capability-checked and produces its own audit row threaded
//! by the original correlation id. The re-applied entry is re-armed on the undo
//! stack, restoring the linear undo/redo invariant.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;

use crate::command::{Command, apply};
use crate::error::{GateError, Result};

use super::stack::{UndoEntry, UndoStore};

/// Redo the last undone definition change for `principal` on `target`.
///
/// Pops the principal + resource redo stack, replays the entry's forward change
/// through the gate (re-checking the capability and writing a new audit row),
/// and re-arms the entry on the undo stack. Returns the entry that was redone.
///
/// # Errors
/// Returns [`GateError::NothingToReverse`] if the redo stack for that slot is
/// empty, or any [`GateError`] the gate raises while re-applying the forward
/// change.
pub async fn redo(
    db: &Surreal<Db>,
    store: &mut UndoStore,
    principal: &Principal,
    target: &rubix_core::Id,
) -> Result<UndoEntry> {
    let entry = store
        .pop_redo(principal, target)
        .ok_or_else(|| GateError::NothingToReverse(format!("no redo history for {target}")))?;

    let command = Command::new(
        entry.principal.clone(),
        entry.capability,
        entry.target.clone(),
        entry.change.forward.clone(),
    );
    apply(db, &command, Some(entry.correlation_id.clone())).await?;

    store.rearm_undo(entry.clone());
    Ok(entry)
}
