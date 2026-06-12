//! Inbound spark dispatch: subscribe to spark findings on the bus and turn each
//! into an embedded-agent *run* — a job, not a chat. A finding like "simultaneous
//! heat/cool on AHU-3" arrives as a published spark and activates the `rubix`
//! agent to investigate and (within its priority floor) act, closing the
//! spark → agent loop the rule boards feed.
//!
//! One detached subscriber loop with a watch-channel shutdown, mirroring the
//! board [`crate::scheduler::Scheduler`]. Requires both a bus (the transport)
//! and the agent runtime; `main` only launches it when both are present.

mod job;
mod run;
mod subscribe;

use std::sync::Arc;

use awaken_runtime::AgentRuntime;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::bus::ZenohBus;

/// Owns the dispatch loop and the signal that stops it.
pub struct Dispatcher {
    shutdown: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

impl Dispatcher {
    /// Launch the spark-dispatch loop on the bus, activating `runtime` per
    /// finding. Returns a handle whose [`Dispatcher::shutdown`] stops it.
    pub fn launch(bus: ZenohBus, runtime: Arc<AgentRuntime>) -> Self {
        let (shutdown, rx) = watch::channel(false);
        let handle = tokio::spawn(subscribe::run_dispatch(bus, runtime, rx));
        tracing::info!("spark dispatcher launched: **/spark/** -> agent runs");
        Self { shutdown, handle }
    }

    /// Signal the loop to stop and await it.
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(true);
        let _ = self.handle.await;
    }
}
