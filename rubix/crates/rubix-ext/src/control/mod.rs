//! The JSON-RPC control surface for an extension.
//!
//! The control plane is JSON-RPC: `register` / `configure` / `invoke` / `health`
//! / `lifecycle` (`rubix/docs/sessions/WS-13.md`, SCOPE "Extensions as
//! principals"). Every extension *command* crosses the WS-05 gate, so an
//! extension is audited identically to a user (contract #1): the mutating
//! methods build a [`Command`](rubix_gate::Command) and drive it through
//! [`apply`](rubix_gate::apply), which checks the WS-04 capability grant fail
//! closed, mints/carries the correlation id, captures before/after atomically,
//! and appends the immutable audit row. An out-of-grant call is refused at
//! [`authorize`](authorize::authorize) before any command is applied — no record
//! and no audit row (contract #2). [`health`](health::probe_health) is the one
//! read-only method: it reports liveness without a command, so it writes no audit
//! row.
//!
//! Laid out one verb per file (`rubix/docs/FILE-LAYOUT.md`): [`request`] is the
//! JSON-RPC envelope, [`authorize`] is the fail-closed capability check every
//! mutating method shares, and one file per method routes that method to its
//! effect.

mod authorize;
mod configure;
mod health;
mod invoke;
mod lifecycle;
mod register;
mod request;

pub use configure::configure;
pub use health::{HealthStatus, probe_health};
pub use invoke::invoke;
pub use lifecycle::{LifecycleAction, lifecycle};
pub use register::register;
pub use request::{ControlMethod, ControlOutcome, ControlRequest};
