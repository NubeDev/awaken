//! The in-memory job registry — the long-running-job spine
//! (`rubix/docs/design/BULK-AND-JOBS.md`, "Job registry — in-memory").
//!
//! Decision: an in-memory `Arc<RwLock<HashMap<JobId, Arc<Job>>>>`, not persisted.
//! The edge deploy is a single embedded SurrealDB node, so a durable queue is
//! unjustified complexity for the first cut. **Consequence:** jobs do not survive a
//! restart — after restart the registry is empty, so any ticket resolves to "job
//! unknown" (a safe-fail, the client resubmits). The registry trait-shape is the
//! seam where a SurrealDB-backed `job` table would slot in later.
//!
//! Each [`Job`] carries its namespace + subject (for cap accounting and the
//! ticket's namespace match), a status enum, a [`CancellationToken`], a
//! `tokio::sync::broadcast` sender for fan-out, and a **bounded backlog ring** so a
//! late or reconnecting subscriber replays the most recent frames then tails the
//! broadcast. Fan-out is broadcast (multi-consumer) rather than `mpsc` because more
//! than one observer can exist (a WS consumer and a reconnecting one, or a poll
//! alongside a socket).
//!
//! Limits are explicit because this is a resource-exhaustion surface: max running
//! jobs per principal and per namespace (over-cap → `429`), a bounded ring, and a
//! hard wall-clock timeout. A dropped WS connection does **not** cancel a job — the
//! cancel token is tripped only by `DELETE`, ticket expiry, or timeout, so a
//! half-committed mutation always runs to completion regardless of who is watching.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rubix_core::Id;
use serde_json::Value;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use tokio::sync::{RwLock, broadcast};
use tokio_util::sync::CancellationToken;

use rubix_gate::sweep_expired_job_tickets;

use super::frame::JobFrame;

/// A job's opaque identifier (a fresh UUID minted at registration).
pub type JobId = String;

/// The lifecycle status of a job.
#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    /// In progress: `done` of `total` units finished (`total` is 0 when unknown).
    Running {
        /// Units completed so far.
        done: u64,
        /// Total units, or 0 when the producer cannot size the work up front.
        total: u64,
    },
    /// Finished successfully.
    Completed,
    /// Ended early: timed out, cancelled, or a producer error.
    Failed {
        /// Why the job ended.
        reason: String,
    },
}

impl JobStatus {
    /// Whether the job is still running (not yet terminal).
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self, JobStatus::Running { .. })
    }
}

/// How a job's terminal result reaches the client — the poll hint
/// (`BULK-AND-JOBS.md`, Tier-2 step 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultTransport {
    /// The result is small + fully buffered (CRUD per-item statuses); the status
    /// poll returns it inline.
    Poll,
    /// The result is streamed over WS only (query chunks are never fully
    /// buffered); the poll is status-only and tells the client to consume the WS.
    Stream,
}

impl ResultTransport {
    /// The stable wire string for this transport (the poll's `result_transport`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ResultTransport::Poll => "poll",
            ResultTransport::Stream => "stream",
        }
    }
}

/// Explicit bounds on the job surface — a resource-exhaustion control plane.
#[derive(Debug, Clone, Copy)]
pub struct JobLimits {
    /// Max concurrently *running* jobs one principal (subject) may hold.
    pub max_running_per_principal: usize,
    /// Max concurrently *running* jobs one namespace may hold.
    pub max_running_per_namespace: usize,
    /// Depth of the per-job backlog ring (the reconnect window, OQ2's `K`).
    pub backlog_ring: usize,
    /// Capacity of the broadcast channel (frames buffered for live subscribers).
    pub broadcast_buffer: usize,
    /// Hard wall-clock timeout per job → `Failed { reason: "timeout" }`.
    pub wall_clock: Duration,
    /// Retention after a job goes terminal before the sweeper evicts it (the
    /// grace window during which a completed result stays pollable).
    pub grace: Duration,
}

impl Default for JobLimits {
    fn default() -> Self {
        // Conservative starting numbers (OQ1) — tune against real timeseries scans.
        Self {
            max_running_per_principal: 4,
            max_running_per_namespace: 16,
            backlog_ring: 512,
            broadcast_buffer: 512,
            wall_clock: Duration::from_secs(300),
            grace: Duration::from_secs(60),
        }
    }
}

/// Why a job could not be registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobError {
    /// The principal or namespace is already at its running-job cap (→ `429`).
    OverCapacity,
}

/// The mutable interior of a job, guarded by a sync mutex so the producer's pushes
/// and a subscriber's snapshot serialise (the exactly-once handoff, below).
struct JobInner {
    status: JobStatus,
    /// The bounded reconnect ring of the most recent frames.
    backlog: VecDeque<JobFrame>,
    /// The fully-buffered terminal result for `Poll`-transport jobs (the per-item
    /// status list the poll returns); `None` for `Stream`-transport jobs, whose
    /// rows are never materialised in full.
    result: Option<Vec<Value>>,
    /// When the job became terminal (drives grace-window eviction); `None` while
    /// running.
    terminal_at: Option<Instant>,
}

/// A single registered job: its scope, fan-out plane, and bounded state.
pub struct Job {
    /// The tenant the job runs in (matched against the ticket's namespace).
    pub namespace: String,
    /// The subject that submitted the job (cap accounting + audit/attribution).
    pub subject: String,
    /// How the terminal result reaches the client.
    pub transport: ResultTransport,
    cancel: CancellationToken,
    sender: broadcast::Sender<JobFrame>,
    ring_cap: usize,
    inner: Mutex<JobInner>,
}

/// A point-in-time view a new subscriber starts from: the backlog to replay, the
/// status at snapshot, and a receiver tailing every frame sent *after* the
/// snapshot — taken atomically so no frame is both replayed and tailed (exactly
/// once) and none is missed.
pub struct JobSubscription {
    /// The recent frames to replay before tailing the live stream.
    pub backlog: Vec<JobFrame>,
    /// The job's status at the moment of subscription.
    pub status: JobStatus,
    /// The live tail; frames sent after the snapshot arrive here.
    pub receiver: broadcast::Receiver<JobFrame>,
}

impl Job {
    /// The job's current status.
    #[must_use]
    pub fn status(&self) -> JobStatus {
        self.inner.lock().expect("job inner mutex").status.clone()
    }

    /// The buffered terminal result for a `Poll`-transport job, if any.
    #[must_use]
    pub fn result(&self) -> Option<Vec<Value>> {
        self.inner.lock().expect("job inner mutex").result.clone()
    }

    /// Trip the job's cancellation token (explicit `DELETE`, ticket expiry).
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// Atomically snapshot the backlog + status and subscribe to the live tail.
    ///
    /// Holding `inner` across `subscribe()` + the backlog clone serialises against
    /// every producer push (which also locks `inner` while it appends to the ring
    /// and sends): a frame pushed *before* this call is in the returned backlog and
    /// not in the receiver; a frame pushed *after* is in the receiver and not the
    /// backlog — so a replay-then-tail consumer sees each frame exactly once.
    #[must_use]
    pub fn subscribe(&self) -> JobSubscription {
        let inner = self.inner.lock().expect("job inner mutex");
        let receiver = self.sender.subscribe();
        JobSubscription {
            backlog: inner.backlog.iter().cloned().collect(),
            status: inner.status.clone(),
            receiver,
        }
    }
}

/// The producer handle a job's task drives: it pushes frames and finishes the job.
///
/// Cloneable (it is shared between the work future and the driver that finalises
/// the job); every clone references the same [`Job`].
#[derive(Clone)]
pub struct JobHandle {
    id: JobId,
    job: Arc<Job>,
    cancel: CancellationToken,
    wall_clock: Duration,
}

impl JobHandle {
    /// The job's id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// A clone of the job's cancellation token (for a `select!` arm).
    #[must_use]
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Whether the job has been cancelled (cooperative check between items).
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// The job's hard wall-clock timeout.
    #[must_use]
    pub fn wall_clock(&self) -> Duration {
        self.wall_clock
    }

    /// Push a frame onto the ring and broadcast it, under one lock so subscribers
    /// see it exactly once (see [`Job::subscribe`]). For a `Poll`-transport job,
    /// an `Item` frame is also appended to the buffered terminal result the poll
    /// returns. Bumps `Running.done` for each `Item`.
    fn push(&self, frame: JobFrame) {
        let mut inner = self.job.inner.lock().expect("job inner mutex");
        if let JobFrame::Item { .. } = &frame {
            if let JobStatus::Running { done, .. } = &mut inner.status {
                *done += 1;
            }
            if self.job.transport == ResultTransport::Poll
                && let Some(buffer) = inner.result.as_mut()
            {
                buffer.push(serde_json::to_value(&frame).unwrap_or(Value::Null));
            }
        }
        inner.backlog.push_back(frame.clone());
        while inner.backlog.len() > self.job.ring_cap {
            inner.backlog.pop_front();
        }
        // Held lock + send → atomic w.r.t. a subscriber's snapshot. A send error
        // (no live receivers) is fine: the frame is retained in the ring for replay.
        let _ = self.job.sender.send(frame);
    }

    /// Emit a per-item status frame for a bulk mutation.
    pub fn item(&self, key: String, status: &str, id: Option<String>, error: Option<String>) {
        self.push(JobFrame::Item {
            key,
            status: status.to_owned(),
            id,
            error,
        });
    }

    /// Emit a query result chunk (with columns on the first chunk).
    pub fn chunk(&self, rows: Vec<Value>, columns: Option<Vec<crate::dto::query::ColumnDto>>) {
        self.push(JobFrame::Chunk { rows, columns });
    }

    /// Mark the job completed and emit the terminal `Done` frame.
    pub fn complete(self) {
        {
            let mut inner = self.job.inner.lock().expect("job inner mutex");
            inner.status = JobStatus::Completed;
            inner.terminal_at = Some(Instant::now());
        }
        self.push(JobFrame::Done);
    }

    /// Mark the job failed with `reason` and emit the terminal `Failed` frame.
    pub fn fail(self, reason: String) {
        {
            let mut inner = self.job.inner.lock().expect("job inner mutex");
            inner.status = JobStatus::Failed {
                reason: reason.clone(),
            };
            inner.terminal_at = Some(Instant::now());
        }
        self.push(JobFrame::Failed { reason });
    }
}

/// The shared, cloneable handle to the in-memory job registry.
#[derive(Clone)]
pub struct JobRegistry {
    jobs: Arc<RwLock<HashMap<JobId, Arc<Job>>>>,
    limits: JobLimits,
}

impl JobRegistry {
    /// Build a registry with explicit [`JobLimits`].
    #[must_use]
    pub fn new(limits: JobLimits) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            limits,
        }
    }

    /// The registry's limits (read by the bulk deadline path for the ring etc.).
    #[must_use]
    pub fn limits(&self) -> &JobLimits {
        &self.limits
    }

    /// Register a new running job for `subject` in `namespace`, or refuse with
    /// [`JobError::OverCapacity`] if the per-principal or per-namespace running cap
    /// is already reached (the caller maps that to `429`).
    ///
    /// `transport` selects whether the terminal result is buffered for the poll or
    /// streamed-only; `total` seeds the progress denominator (0 if unknown).
    pub async fn try_register(
        &self,
        namespace: &str,
        subject: &str,
        transport: ResultTransport,
        total: u64,
    ) -> Result<JobHandle, JobError> {
        let mut jobs = self.jobs.write().await;

        let (mut per_principal, mut per_namespace) = (0usize, 0usize);
        for job in jobs.values() {
            if !job.status().is_running() || job.namespace != namespace {
                continue;
            }
            per_namespace += 1;
            if job.subject == subject {
                per_principal += 1;
            }
        }
        if per_principal >= self.limits.max_running_per_principal
            || per_namespace >= self.limits.max_running_per_namespace
        {
            return Err(JobError::OverCapacity);
        }

        let id = Id::new().as_str().to_owned();
        let (sender, _rx) = broadcast::channel(self.limits.broadcast_buffer);
        let cancel = CancellationToken::new();
        let job = Arc::new(Job {
            namespace: namespace.to_owned(),
            subject: subject.to_owned(),
            transport,
            cancel: cancel.clone(),
            sender,
            ring_cap: self.limits.backlog_ring,
            inner: Mutex::new(JobInner {
                status: JobStatus::Running { done: 0, total },
                backlog: VecDeque::new(),
                result: match transport {
                    ResultTransport::Poll => Some(Vec::new()),
                    ResultTransport::Stream => None,
                },
                terminal_at: None,
            }),
        });
        jobs.insert(id.clone(), job.clone());
        Ok(JobHandle {
            id,
            job,
            cancel,
            wall_clock: self.limits.wall_clock,
        })
    }

    /// Fetch a job by id (for a poll, a WS subscribe, or a cancel).
    pub async fn get(&self, id: &str) -> Option<Arc<Job>> {
        self.jobs.read().await.get(id).cloned()
    }

    /// Evict every terminal job whose grace window has elapsed, and reap expired
    /// ticket rows. Returns how many jobs were evicted.
    ///
    /// Eviction does **not** eagerly revoke the evicted job's ticket (see the
    /// "Decisions taken during build" note in `BULK-AND-JOBS.md`): once the job is
    /// gone from the registry, [`resolve_observer`](super::resolve_observer) returns
    /// "job unknown" (`404`) regardless, so a post-eviction poll gets the *same*
    /// fail-closed answer as the restart/job-absent path rather than an ambiguous
    /// `401`. The now-orphan ticket row resolves to `404` for its short remaining
    /// TTL and is reaped by the expired-row sweep below — no security or resource
    /// cost. Explicit `DELETE` still revokes the ticket eagerly.
    ///
    /// Called periodically by the spawned sweeper; tests call it directly (with a
    /// zero grace) to assert the grace-window eviction without sleeping.
    pub async fn sweep(&self, db: &Surreal<Db>) -> u64 {
        let now = Instant::now();
        let mut evict = Vec::new();
        {
            let jobs = self.jobs.read().await;
            for (id, job) in jobs.iter() {
                let terminal_at = job.inner.lock().expect("job inner mutex").terminal_at;
                if let Some(at) = terminal_at
                    && now.saturating_duration_since(at) >= self.limits.grace
                {
                    evict.push(id.clone());
                }
            }
        }
        if !evict.is_empty() {
            let mut jobs = self.jobs.write().await;
            for id in &evict {
                jobs.remove(id);
            }
        }
        // Best-effort: reap expired ticket rows (incl. now-orphaned ones once their
        // TTL lapses); a sweep failure must not abort the eviction.
        let _ = sweep_expired_job_tickets(db).await;
        evict.len() as u64
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new(JobLimits::default())
    }
}

/// Drive `work` as a background job behind `handle`, with a hard wall-clock
/// timeout and cancellation, finalising the job with the terminal frame.
///
/// The work future races the cancel token and the wall-clock sleep: cancellation
/// (explicit `DELETE` / ticket expiry) or timeout ends the job `Failed`, otherwise
/// the work's own `Ok`/`Err` decides `Done`/`Failed`. The work future receives a
/// clone of the handle to emit progress/item/chunk frames; it should also poll
/// [`JobHandle::is_cancelled`] between units so a cancel stops it promptly rather
/// than only at the next await point.
pub fn drive<F, Fut>(handle: JobHandle, work: F)
where
    F: FnOnce(JobHandle) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
{
    let cancel = handle.cancel_token();
    let wall_clock = handle.wall_clock();
    let worker = handle.clone();
    tokio::spawn(async move {
        let outcome = tokio::select! {
            biased;
            () = cancel.cancelled() => Err("cancelled".to_owned()),
            () = tokio::time::sleep(wall_clock) => Err("timeout".to_owned()),
            result = work(worker) => result,
        };
        match outcome {
            Ok(()) => handle.complete(),
            Err(reason) => handle.fail(reason),
        }
    });
}
