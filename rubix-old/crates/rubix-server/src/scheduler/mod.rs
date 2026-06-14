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
use rubix_datasource::DatasourceRegistry;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use uuid::Uuid;

use self::evaluate::BoardRunDeps;
use crate::bus::ZenohBus;
use crate::store::Store;

/// One running board loop: its task handle and a per-board shutdown switch, so
/// it can be cancelled independently when the board is disabled or republished.
struct BoardTask {
    shutdown: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

/// A board's **stable** identity across versions: `(org, site_id, slug)`. The
/// row `id` changes on every republish (each version is a new row), so loops and
/// node state must key on this, not on `id` — otherwise a republish leaves the
/// old version's loop running (the board fires once per save) and resets the
/// board's node state (a trigger re-boots).
pub(super) type BoardKey = (String, Option<Uuid>, String);

/// The stable key for a board record.
pub(super) fn board_key(board: &BoardRecord) -> BoardKey {
    (board.org.clone(), board.site_id, board.slug.clone())
}

/// The stable key rendered for node-state scoping (the store's `board_id` column
/// and the in-memory session map). Stable across versions.
pub(super) fn board_state_key(org: &str, site_id: Option<Uuid>, slug: &str) -> String {
    format!(
        "{org}\u{1f}{}\u{1f}{slug}",
        site_id.map(|s| s.to_string()).unwrap_or_default()
    )
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
    /// External datasources backing `datasource` nodes in scheduled boards.
    /// `None` when no manifest is loaded; such a board's `datasource` node fails
    /// closed at run time.
    datasources: Option<Arc<DatasourceRegistry>>,
    outputs: BoardOutputs,
    /// Process-wide, board-scoped `Session` node state. Owned here so it outlives
    /// an engine rebuild (a republish/save) — that is what lets a `trigger`'s
    /// clock survive a save instead of re-firing its boot fire.
    session_state: crate::flow::SessionStore,
    /// Live loops keyed by the board's **stable** identity (`(org, site_id,
    /// slug)`), not its per-version row id — so republishing a board (a new
    /// version, new id) cancels and replaces its existing loop instead of leaving
    /// the old one running.
    tasks: Mutex<HashMap<BoardKey, BoardTask>>,
}

impl SchedulerInner {
    /// The backend services every scheduled board run binds to. Cloned per loop
    /// so each detached task owns its own handles.
    fn board_run_deps(&self) -> BoardRunDeps {
        BoardRunDeps {
            store: self.store.clone(),
            bus: self.bus.clone(),
            agent: self.agent.clone(),
            datasources: self.datasources.clone(),
            outputs: self.outputs.clone(),
            session_state: self.session_state.clone(),
        }
    }
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
        datasources: Option<Arc<DatasourceRegistry>>,
        boards: Vec<BoardRecord>,
    ) -> Self {
        let scheduler = Self {
            inner: Arc::new(SchedulerInner {
                store,
                bus,
                agent,
                datasources,
                outputs: BoardOutputs::new(),
                session_state: Arc::new(std::sync::Mutex::new(HashMap::new())),
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

    /// Register (or re-register) a board's loop, keyed by its **stable** identity
    /// (`(org, site_id, slug)`). Any existing loop for that identity is cancelled
    /// first, so republishing (a new version, new row id) replaces the old loop
    /// rather than leaving it running — without this, every save spawns another
    /// loop and the board fires once per save. A non-scheduled (manual or
    /// disabled) board is unregistered instead. The loop re-fetches the *latest*
    /// version by `(org, site_id, slug)` each tick, so a republish takes effect
    /// without re-registering.
    pub fn register(&self, board: &BoardRecord) {
        if !board.is_scheduled() {
            self.unregister(board);
            return;
        }
        // Cancel any prior loop for this stable identity before spawning the new.
        self.unregister(board);

        let (shutdown, rx) = watch::channel(false);
        let inner = self.inner.clone();
        let key = board_key(board);
        let (org, site_id, slug) = (board.org.clone(), board.site_id, board.slug.clone());
        let deps = inner.board_run_deps();
        let handle = match board.trigger.clone() {
            Trigger::Manual => return, // filtered out by is_scheduled
            Trigger::Interval { seconds } => tokio::spawn(interval::run_interval(
                org,
                site_id,
                slug.clone(),
                seconds,
                deps,
                rx,
            )),
            Trigger::Subscription { key: sub_key } => match &inner.bus {
                // The bus is required for the `watch` subscription; `deps` carries
                // it, so the loop declares the watch through the seam itself.
                Some(_) => tokio::spawn(subscribe::run_subscription(
                    org,
                    site_id,
                    slug.clone(),
                    sub_key,
                    deps,
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
            tasks.insert(key, BoardTask { shutdown, handle });
        }
        tracing::info!(board = %slug, "board loop registered");
    }

    /// Stop and drop a board's loop, if any (keyed by stable identity).
    /// Idempotent. Also clears the board's cached outputs (keyed by slug) so a
    /// disabled/deleted board does not show stale values.
    pub fn unregister(&self, board: &BoardRecord) {
        let key = board_key(board);
        let task = self.inner.tasks.lock().ok().and_then(|mut t| t.remove(&key));
        if let Some(task) = task {
            let _ = task.shutdown.send(true);
            task.handle.abort();
            tracing::info!(board = %board.slug, "board loop unregistered");
        }
        self.inner.outputs.clear(&board.slug);
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
