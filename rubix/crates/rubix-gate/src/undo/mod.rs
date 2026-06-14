//! Undo/redo — reversible change records for user-facing definitions.
//!
//! `rubix/docs/SCOPE.md`, "Undo/redo" (contract #4 in `rubix/STACK-DEISGN.md`):
//! definition mutations (dashboards, rules, tags, datasource config) produce a
//! reversible change record — a forward plus an inverse — derived from the *same*
//! capture the audit log projects from (one capture, two consumers). Undo is the
//! **mutable** consumer: a per-principal + resource stack pushed on a definition
//! mutation and popped on undo, with a redo stack fed by undo. A reversal re-enters
//! the WS-05 gate ([`apply`](crate::command::apply)) as a fresh command, so it is
//! capability-checked and itself audited. The boundary is hard: undo covers
//! definitions only — never the data plane and never the audit log
//! ([`classify`](classify)). The pipeline is split one verb per file:
//! [`classify`] the boundary, [`change`] the inverse, [`push`] onto the stack,
//! [`reverse`] the inverse through the gate, and [`redo`] the forward.

mod change;
mod classify;
mod push;
mod redo;
mod reverse;
mod stack;

pub use change::ChangeRecord;
pub use classify::{RecordKind, is_undoable};
pub use push::push_change;
pub use redo::redo;
pub use reverse::undo;
pub use stack::{UndoEntry, UndoStore};
