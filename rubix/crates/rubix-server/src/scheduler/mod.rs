//! Board scheduler: fires stored boards on a cadence (`Interval`) or off a
//! live `cur` subscription (`Subscription`), turning `/boards/run`'s caller-
//! driven evaluation into an autonomous control/rule loop. One detached task
//! per scheduled board, each with its own cancellation, so a board can be
//! registered or torn down at runtime without touching the others.
//!
//! The scheduler holds no graphs — each loop re-reads its board from the store
//! when its trigger fires, so republishing a board version or disabling it
//! takes effect on the next tick/sample without restarting the scheduler. It
//! does hold the [`BoardOutputs`] cache so every run's node outputs surface to
//! clients regardless of which loop produced them.

mod evaluate;
mod interval;
mod outputs;
mod record;
mod subscribe;
mod trigger;

pub use outputs::{BoardOutputs, PortOutput};
pub use record::BoardRecord;
pub use trigger::Trigger;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use awaken_runtime::AgentRuntime;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::bus::ZenohBus;
use crate::store::Store;

/// One running board loop: its task handle and a per-board shutdown switch, so
/// it can be cancelled independently when the board is disabled or republished.
struct BoardTask {
    shutdown: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

/// Owns the per-board scheduling tasks and the in-memory output cache. Cheaply
/// cloneable for sharing in `AppState`: the task table and cache are behind an
/// `Arc`, so all clones drive the same scheduler.
#[derive(Clone)]
pub struct Scheduler {
    inner: Arc<SchedulerInner>,
}

struct SchedulerInner {
    store: Store,
    bus: Option<ZenohBus>,
    agent: Option<Arc<AgentRuntime>>,
    outputs: BoardOutputs,
    tasks: Mutex<HashMap<String, BoardTask>>,
}

impl Scheduler {
    /// Launch a loop for every scheduled board currently in the store. The bus,
    /// when present, backs subscription triggers and `emit_spark` publishing;
    /// without it, subscription boards are skipped with a warning (interval
    /// boards still run, and their sparks persist without publishing). The
    /// agent, when present, backs `agent_call` nodes in scheduled boards.
    pub fn launch(
        store: Store,
        bus: Option<ZenohBus>,
        agent: Option<Arc<AgentRuntime>>,
        boards: Vec<BoardRecord>,
    ) -> Self {
        let scheduler = Self {
            inner: Arc::new(SchedulerInner {
                store,
                bus,
                agent,
                outputs: BoardOutputs::new(),
                tasks: Mutex::new(HashMap::new()),
            }),
        };
        for board in boards.into_iter().filter(BoardRecord::is_scheduled) {
            scheduler.register(&board);
        }
        tracing::info!(boards = scheduler.active(), "board scheduler launched");
        scheduler
    }

    /// The shared output cache, for handlers that record on-demand runs or read
    /// the latest values an enabled board produced.
    pub fn outputs(&self) -> &BoardOutputs {
        &self.inner.outputs
    }

    /// Number of live scheduling loops.
    pub fn active(&self) -> usize {
        self.inner.tasks.lock().map(|t| t.len()).unwrap_or(0)
    }

    /// Register (or re-register) a board's loop. Any existing loop for the slug
    /// is cancelled first, so a republished or re-enabled board picks up its
    /// new cadence/graph immediately rather than at the next restart. A
    /// non-scheduled (manual, or disabled) board is unregistered instead — its
    /// presence here is what "enabled" means at runtime.
    pub fn register(&self, board: &BoardRecord) {
        if !board.is_scheduled() {
            self.unregister(&board.slug);
            return;
        }
        // Cancel any prior loop for this slug before spawning the new one.
        self.unregister(&board.slug);

        let (shutdown, rx) = watch::channel(false);
        let inner = self.inner.clone();
        let slug = board.slug.clone();
        let handle = match board.trigger.clone() {
            Trigger::Manual => return, // filtered out by is_scheduled
            Trigger::Interval { seconds } => tokio::spawn(interval::run_interval(
                slug.clone(),
                seconds,
                inner.store.clone(),
                inner.bus.clone(),
                inner.agent.clone(),
                inner.outputs.clone(),
                rx,
            )),
            Trigger::Subscription { key } => match &inner.bus {
                Some(bus) => tokio::spawn(subscribe::run_subscription(
                    slug.clone(),
                    key,
                    bus.clone(),
                    inner.store.clone(),
                    inner.agent.clone(),
                    inner.outputs.clone(),
                    rx,
                )),
                None => {
                    tracing::warn!(
                        board = %slug,
                        "subscription board skipped: no zenoh session (RUBIX_ZENOH=0)"
                    );
                    return;
                }
            },
        };
        if let Ok(mut tasks) = self.inner.tasks.lock() {
            tasks.insert(slug.clone(), BoardTask { shutdown, handle });
        }
        tracing::info!(board = %slug, "board loop registered");
    }

    /// Stop and drop a board's loop, if any. Idempotent. Also clears the board's
    /// cached outputs so a disabled/deleted board does not show stale values.
    pub fn unregister(&self, slug: &str) {
        let task = self.inner.tasks.lock().ok().and_then(|mut t| t.remove(slug));
        if let Some(task) = task {
            let _ = task.shutdown.send(true);
            task.handle.abort();
            tracing::info!(board = %slug, "board loop unregistered");
        }
        self.inner.outputs.clear(slug);
    }

    /// Signal every board loop to stop and await them.
    pub async fn shutdown(self) {
        let tasks: Vec<BoardTask> = self
            .inner
            .tasks
            .lock()
            .map(|mut t| t.drain().map(|(_, v)| v).collect())
            .unwrap_or_default();
        for task in tasks {
            let _ = task.shutdown.send(true);
            let _ = task.handle.await;
        }
    }
}
