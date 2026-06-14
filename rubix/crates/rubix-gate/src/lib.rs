//! Access & policy gate for the rubix platform.
//!
//! The read enforcement point: reads run on a gate-issued **scoped SurrealDB
//! session** enforced by SurrealDB row-level permissions, never proxied per
//! message (`rubix/STACK-DEISGN.md`, contracts #1 and #2; `rubix/docs/SCOPE.md`,
//! principles 5 and 7). One identity model — `rubix_core::Principal` — for users
//! and extensions; this crate authenticates a principal, mints its scoped
//! session, and runs scoped reads. The capability-grant layer (app-enforced
//! authz over cross-plane actions, the second authz layer of
//! `rubix/docs/SCOPE.md`) lives in [`capability`]; the command path lands in a
//! later workstream.

mod authenticate;
pub mod capability;
mod error;
mod permission;
mod principal;
mod read;
mod session;
mod token;

pub use authenticate::authenticate;
pub use capability::{
    Capability, Grant, check_capability, create_grant, is_registered, list_grants, revoke_grant,
};
pub use error::{GateError, Result};
pub use permission::define_gate_schema;
pub use principal::provision_principal;
pub use read::{read_record_on_session, read_records_on_session};
pub use session::{ScopedSession, issue_scoped_session};
pub use token::PrincipalToken;
