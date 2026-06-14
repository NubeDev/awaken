//! Capability grants — the second authz layer (app-enforced).
//!
//! `rubix/docs/SCOPE.md` ("Two authz layers"): data-record reads are scoped by
//! SurrealDB row-level permissions (WS-03); cross-plane actions — register a
//! datasource, invoke a rule, publish ingest, query an external source,
//! subscribe a Zenoh key-space — are governed here, by app-enforced grants.
//! Both layers key off the same [`Principal`](rubix_core::Principal). This layer
//! fails closed: an unknown capability and a missing grant both deny.

mod check;
pub mod grant;
mod kind;
mod register;

pub use check::check_capability;
pub use grant::{Grant, create_grant, list_grants, revoke_grant};
pub use kind::Capability;
pub use register::is_registered;
