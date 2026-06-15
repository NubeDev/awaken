//! Provision an extension as a scoped service-account principal.
//!
//! An extension is *not* a separate plugin-trust path: it is provisioned on the
//! **same identity model** as a user (`rubix/docs/sessions/WS-13.md`, SCOPE
//! "Extensions as principals"; `rubix/docs/SCOPE.md`, principle 5) — a
//! [`Principal`](rubix_core::Principal) whose [`kind`](rubix_core::PrincipalKind)
//! is `Extension`, bound to one namespace. [`register_extension`] writes that
//! identity through the WS-03 provisioning path so the extension can later
//! authenticate and sign in to a scoped session exactly as a user does;
//! [`grant_extension`] attaches the WS-04 capability grants that decide what the
//! extension may *do*. The two layers stay distinct: SurrealDB row-perms scope
//! the data it can read, capability grants scope the cross-plane actions it can
//! invoke (`rubix/docs/sessions/WS-13.md`, contract #2).

mod grant;
mod register;

pub use grant::{GrantProfile, grant_extension};
pub use register::{ExtensionRegistration, register_extension};
