//! Audit log — append-only, immutable, correlation-id stamped.
//!
//! `rubix/docs/SCOPE.md` ("Audit log"; contract #4 in `rubix/STACK-DEISGN.md`):
//! every mutating command produces one immutable audit row captured at the gate
//! — who did what, when, with the before/after summary and the correlation id.
//! Immutability is enforced by SurrealDB table permissions, not app discipline
//! (see [`permit`]); the row is the immutable projection of the one captured
//! change the gate takes, distinct from the mutable undo stack (WS-06).

mod append;
mod permit;
mod record;

pub(crate) use append::append_audit;
pub use permit::define_audit_schema;
pub use record::AuditRecord;
