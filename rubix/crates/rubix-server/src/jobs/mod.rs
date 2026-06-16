//! The long-running job spine: the in-memory registry, the typed WS frames, and
//! the periodic sweeper (`rubix/docs/design/BULK-AND-JOBS.md`, "The job spine").
//!
//! Both bulk use cases (record CRUD, streaming query) are thin layers over this
//! shared infra rather than two divergent one-offs: a job is registered, driven on
//! a `tokio::spawn`, observed over the WS plane or the status poll, and evicted
//! after a grace window. This barrel re-exports the spine's public surface and
//! owns the sweeper task the binary spawns at boot.

mod access;
mod frame;
mod registry;

use std::time::Duration;

pub use access::{
    MintedTicket, SYNTHETIC_STEP_DELAY, mint_ticket, register_job, require_bulk_submit,
    resolve_observer,
};
pub use frame::JobFrame;
pub use registry::{
    Job, JobError, JobHandle, JobId, JobLimits, JobRegistry, JobStatus, JobSubscription,
    ResultTransport, drive,
};

use crate::state::AppState;

/// Spawn the background job sweeper: it periodically evicts terminal jobs whose
/// grace window has elapsed (revoking each one's ticket) and reaps expired ticket
/// rows, bounding the registry's and the ticket table's growth.
///
/// Mirrors [`spawn_hook_dispatcher`](crate::spawn_hook_dispatcher): the binary
/// calls this once after building state. The interval is half the grace window
/// (clamped to a sane floor) so a terminal job is reliably evicted within roughly
/// one grace window of finishing. Tests drive [`JobRegistry::sweep`] directly
/// instead, so they do not depend on this loop's cadence.
pub fn spawn_job_sweeper(state: AppState) {
    let interval = state
        .jobs
        .limits()
        .grace
        .checked_div(2)
        .unwrap_or(Duration::from_secs(30))
        .max(Duration::from_secs(5));
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            state.jobs.sweep(state.store.raw()).await;
        }
    });
}
