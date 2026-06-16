//! Access & policy gate for the rubix platform.
//!
//! The read enforcement point: reads run on a gate-issued **scoped SurrealDB
//! session** enforced by SurrealDB row-level permissions, never proxied per
//! message (`rubix/STACK-DEISGN.md`, contracts #1 and #2; `rubix/docs/SCOPE.md`,
//! principles 5 and 7). One identity model — `rubix_core::Principal` — for users
//! and extensions; this crate authenticates a principal, mints its scoped
//! session, and runs scoped reads. The capability-grant layer (app-enforced
//! authz over cross-plane actions, the second authz layer of
//! `rubix/docs/SCOPE.md`) lives in [`capability`]. The write-enforcement point —
//! every mutation crosses the gate as a [`Command`], which checks the grant,
//! captures before/after atomically, mints/carries the correlation id, applies
//! the change, and writes an immutable audit row — lives in [`command`] and
//! [`audit`] (contracts #1, #3, #4).

mod audit;
mod auth_token;
mod authenticate;
pub mod capability;
mod command;
mod error;
mod permission;
mod principal;
mod read;
mod session;
mod tenant;
mod token;
mod undo;

pub use audit::{AuditRecord, define_audit_schema};
pub use auth_token::{
    DEFAULT_TTL_SECONDS, IssuedToken, ResolvedToken, issue_session_token, resolve_session_token,
    revoke_session_token,
};
pub use authenticate::authenticate;
pub use capability::{
    Capability, Grant, check_capability, create_grant, create_grant_audited, is_registered,
    list_grants, revoke_grant, revoke_grant_audited,
};
pub use command::{Applied, CapturedChange, Change, Command, apply};
pub use error::{GateError, Result};
pub use permission::define_gate_schema;
pub use principal::{
    create_principal, delete_principal, get_principal, list_principals, provision_principal,
    set_principal_role,
};
pub use read::{
    read_readings_on_session, read_record_on_session, read_record_tags_on_session,
    read_records_on_session, read_records_on_session_filtered,
};
pub use session::{ScopedSession, issue_scoped_session};
pub use tenant::purge_namespace;
pub use token::PrincipalToken;
pub use undo::{ChangeRecord, RecordKind, UndoEntry, UndoStore, is_undoable, push_change, redo, undo};
