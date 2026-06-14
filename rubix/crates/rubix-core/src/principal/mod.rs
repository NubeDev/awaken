//! Principal identity: the one model for users and extensions.
//!
//! Crate role (`rubix/STACK-DEISGN.md`, `rubix-core` row): rubix-core owns the
//! principal identity types. Enforcement (authenticate, scoped-session issuance,
//! row-level permissions) lives in `rubix-gate`; the capability layer is a later
//! workstream — both build on this type.

mod identity;
mod kind;
mod role;

pub use identity::Principal;
pub use kind::PrincipalKind;
pub use role::Role;
