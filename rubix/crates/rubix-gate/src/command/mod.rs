//! Command gate — the single write-enforcement point.
//!
//! Contract #1 (`rubix/STACK-DEISGN.md`; `rubix/docs/SCOPE.md`, "Commands go
//! through the gate"): every mutation crosses the gate as a [`Command`], which
//! authenticates the principal (resolved upstream), checks its capability grant
//! (WS-04), mints/carries the correlation id, captures before/after atomically
//! with the write via SurrealDB `RETURN BEFORE`, applies the change, and writes
//! the immutable audit row. The pipeline is split one verb (phase) per file:
//! [`authorize`] the grant, [`correlate`] the id, [`capture`]+[`persist`] the
//! atomic mutation, and [`apply`] orchestrates them with the audit append.

mod action;
mod apply;
mod authorize;
mod capture;
mod correlate;
mod define;
mod persist;

pub use action::Change;
pub use apply::{Applied, apply};
pub use capture::CapturedChange;
pub use define::Command;
