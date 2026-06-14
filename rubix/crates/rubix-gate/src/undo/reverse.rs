//! Undo the last definition mutation by replaying its inverse through the gate.
//!
//! `rubix/docs/SCOPE.md`, "Undo/redo": undo applies the inverse **through the
//! gate**, so it is permission-checked and itself audited. This verb pops the
//! most recent undoable entry for a principal + resource, constructs a WS-05
//! [`Command`] from the entry's inverse change, and drives it through
//! [`apply`](crate::command::apply) carrying the original correlation id — so the
//! reversal re-runs the capability check, captures its own before/after, and
//! produces a fresh audit row threaded to the same chain. The consumed entry then
//! moves onto the redo stack so the forward change can be re-applied.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;

use crate::command::{Command, apply};
use crate::error::{GateError, Result};

use super::stack::{UndoEntry, UndoStore};

/// Undo the last definition change for `principal` on `target`.
///
/// Pops the principal + resource undo stack, replays the entry's inverse change
/// through the gate (re-checking the capability and writing a new audit row),
/// and pushes the entry onto the redo stack. Returns the entry that was undone.
///
/// # Errors
/// Returns [`GateError::NothingToReverse`] if the undo stack for that slot is
/// empty, or any [`GateError`] the gate raises while applying the inverse (a
/// denied capability or a failed write leaves the entry off both stacks, so the
/// caller sees the failure rather than a silently lost step).
pub async fn undo(
    db: &Surreal<Db>,
    store: &mut UndoStore,
    principal: &Principal,
    target: &rubix_core::Id,
) -> Result<UndoEntry> {
    let entry = store
        .pop_undo(principal, target)
        .ok_or_else(|| GateError::NothingToReverse(format!("no undo history for {target}")))?;

    let command = Command::new(
        entry.principal.clone(),
        entry.capability,
        entry.target.clone(),
        entry.change.inverse.clone(),
    );
    apply(db, &command, Some(entry.correlation_id.clone())).await?;

    store.push_redo(entry.clone());
    Ok(entry)
}
