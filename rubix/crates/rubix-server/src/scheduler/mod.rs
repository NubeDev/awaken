//! Board scheduler: fires stored boards on a cadence (`Interval`) or off a
//! live `cur` subscription (`Subscription`), turning `/boards/run`'s caller-
//! driven evaluation into an autonomous control/rule loop. One detached task
//! per scheduled board; all share a watch-channel shutdown, mirroring the
//! driver [`crate::supervisor::Supervisor`].
//!
//! The scheduler holds no graphs — each loop re-reads its board from the store
//! when its trigger fires, so republishing a board version or disabling it
//! takes effect on the next tick/sample without restarting the scheduler.

mod evaluate;
mod interval;
mod record;
mod subscribe;
mod trigger;

pub use record::BoardRecord;
pub use trigger::Trigger;

use std::sync::Arc;

use awaken_runtime::AgentRuntime;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::bus::ZenohBus;
use crate::store::Store;

/// Owns the per-board scheduling tasks and the signal that stops them.
pub struct Scheduler {
    shutdown: watch::Sender<bool>,
    handles: Vec<JoinHandle<()>>,
}

impl Scheduler {
    /// Launch a loop for every scheduled board currently in the store. The bus,
    /// when present, backs subscription triggers and `emit_spark` publishing;
    /// without it, subscription boards are skipped with a warning (interval
    /// boards still run, and their sparks persist without publishing). Returns
    /// a handle whose [`Scheduler::shutdown`] stops every loop. The agent, when
    /// present, backs `agent_call` nodes in scheduled boards.
    pub fn launch(
        store: Store,
        bus: Option<ZenohBus>,
        agent: Option<Arc<AgentRuntime>>,
        boards: Vec<BoardRecord>,
    ) -> Self {
        let (shutdown, rx) = watch::channel(false);
        let mut handles = Vec::new();
        for board in boards.into_iter().filter(BoardRecord::is_scheduled) {
            match board.trigger {
                Trigger::Manual => {} // filtered out by is_scheduled
                Trigger::Interval { seconds } => {
                    handles.push(tokio::spawn(interval::run_interval(
                        board.slug,
                        seconds,
                        store.clone(),
                        bus.clone(),
                        agent.clone(),
                        rx.clone(),
                    )));
                }
                Trigger::Subscription { key } => match &bus {
                    Some(bus) => {
                        handles.push(tokio::spawn(subscribe::run_subscription(
                            board.slug,
                            key,
                            bus.clone(),
                            store.clone(),
                            agent.clone(),
                            rx.clone(),
                        )));
                    }
                    None => {
                        tracing::warn!(
                            board = %board.slug,
                            "subscription board skipped: no zenoh session (RUBIX_ZENOH=0)"
                        );
                    }
                },
            }
        }
        tracing::info!(boards = handles.len(), "board scheduler launched");
        Self { shutdown, handles }
    }

    /// Number of live scheduling loops.
    pub fn active(&self) -> usize {
        self.handles.len()
    }

    /// Signal every board loop to stop and await them.
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(true);
        for h in self.handles {
            let _ = h.await;
        }
    }
}
