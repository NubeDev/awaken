//! The extension **runtime supervisor** — the layer that makes an extension run.
//!
//! `rubix-ext` models an extension as a scoped principal with capability grants
//! (the identity/authorization half). This module is the **runtime half**
//! (`rubix/docs/design/EXTENSION-RUNTIME.md`): given a process-flavour extension,
//! it spawns and supervises a child process under that extension's identity,
//! restarts it on crash with backoff, samples its process stats, and surfaces a
//! bounded event ring of what happened. It is ported from
//! `starter-ext-supervisor` and re-skinned onto rubix's own SPI types — a rubix
//! [`ExtensionId`] keyed off the gate's `Principal`, not a manifest id space.
//!
//! The mechanism is pure OS plumbing with no authorization opinion: *who* may
//! drive a lifecycle transition is decided one layer up, at the gate
//! ([`crate::lifecycle`] checks [`ExtensionManage`](rubix_gate::Capability::ExtensionManage)
//! fail closed before the supervisor is ever touched). The supervisor is a
//! *subscriber* to that decision, never an authority over it (`rubix/docs/design/
//! EXTENSION-RUNTIME.md`, "the gate boundary").
//!
//! Laid out one concern per file (`rubix/docs/FILE-LAYOUT.md`):
//!
//! - [`id`] — the per-extension key derived from the principal.
//! - [`state`] / [`flavour`] — the runtime-state and packaging-flavour vocabulary.
//! - [`spec`] — how to spawn one child (read off the gated config record).
//! - [`stats`] — `ProcessStats` + `/proc` sampling.
//! - [`ring`] — the bounded per-extension event ring.
//! - [`backoff`] / [`restart`] — the restart-policy state machine.
//! - [`stdio`] — `Content-Length`-framed JSON-RPC over the child's pipes.
//! - [`handle`] / [`task`] — the live handle and the spawn/serve/restart loop.
//! - [`registry`] — the in-memory `ExtensionId → handle` map.

mod backoff;
mod flavour;
mod handle;
mod id;
mod registry;
mod restart;
mod ring;
mod spec;
mod state;
mod stats;
mod stdio;
mod task;

pub use backoff::Backoff;
pub use flavour::ProcessFlavour;
pub use handle::SupervisorHandle;
pub use id::ExtensionId;
pub use registry::SupervisorRegistry;
pub use restart::{ExitReason, RestartDecision, RestartPolicy, RestartTracker};
pub use ring::{DEFAULT_CAPACITY, Event, EventKind, EventRing, MAX_STDERR_LINE_BYTES};
pub use spec::{HealthConfig, ProcessSpec};
pub use state::LifecycleState;
pub use stats::ProcessStats;
pub use task::{Identity, Supervisor};
