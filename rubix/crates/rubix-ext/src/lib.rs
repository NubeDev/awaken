//! Extensions as scoped principals for the rubix platform.
//!
//! An extension is modelled as a **service account on the same identity model as
//! a user** (`rubix/docs/sessions/WS-13.md`, SCOPE "Extensions as principals";
//! `rubix/docs/SCOPE.md`, principle 5) — a scoped
//! [`Principal`](rubix_core::Principal) of kind `Extension` bound to one
//! namespace, not a privileged plugin-trust path. Enforcement is the two layers
//! shared with users: SurrealDB row-level permissions scope the data records the
//! extension can read (WS-03), and app-enforced WS-04 capability grants scope the
//! cross-plane actions it can take (`rubix/docs/sessions/WS-13.md`, contract #2).
//!
//! The crate has three faces:
//!
//! - [`provision`] registers an extension as a scoped service-account principal
//!   ([`register_extension`]) and attaches its capability grants
//!   ([`grant_extension`]) — the same grant mechanism expresses a read-only, an
//!   ingest-only, and an admin extension; only the [`GrantProfile`] differs.
//! - [`control`] is the JSON-RPC control plane — `register` / `configure` /
//!   `invoke` / `health` / `lifecycle`. Every mutating method crosses the WS-05
//!   gate as a [`Command`](rubix_gate::Command), so an extension command is
//!   capability-checked, correlated, and audited identically to a user's
//!   (contract #1); an out-of-grant call is denied before any effect.
//! - [`data`] delegates the data plane to WS-12's Zenoh key-space scoping
//!   ([`authorize_data_scope`]) — scope resolved once at subscribe, not
//!   re-implemented here.

mod control;
mod data;
mod error;
mod provision;

pub use control::{
    ControlMethod, ControlOutcome, ControlRequest, HealthStatus, LifecycleAction, configure,
    invoke, lifecycle, probe_health, register,
};
pub use data::authorize_data_scope;
pub use error::{ExtError, Result};
pub use provision::{ExtensionRegistration, GrantProfile, grant_extension, register_extension};
