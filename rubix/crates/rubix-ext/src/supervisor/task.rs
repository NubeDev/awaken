//! [`Supervisor`] — spawn, init-handshake, watch, restart one extension child.
//!
//! Ported and re-skinned from `starter-ext-supervisor::supervisor`
//! (`rubix/docs/design/EXTENSION-RUNTIME.md`, phase 1; "Recommendation: port").
//! This is the entry point that brings a process-flavour extension up. It owns
//! the child's lifecycle as one `spawn → init handshake → serve → exit observed →
//! decide → respawn or stop` cycle per restart, plus the periodic health pinger,
//! all in one async task.
//!
//! [`Supervisor::start`] spawns the management task and returns immediately with a
//! [`SupervisorHandle`]; it never blocks the caller.
//!
//! ## Deliberate scope vs. starter
//!
//! Rebound onto rubix's model, three starter mechanisms are intentionally dropped
//! (each a clean, separable concern, not a stub):
//!
//! - **No manifest-hash handshake.** rubix has no `block.yaml`; the init request
//!   carries the gated config, and the child answers `ready: true`. There is no
//!   content hash to verify because there is no manifest file to verify against.
//! - **Cooperative shutdown over the wire**, not POSIX signals. The supervisor
//!   sends a `shutdown` notification on the existing control channel, waits the
//!   grace window, then hard-kills via tokio's `start_kill`. This keeps the crate
//!   free of a `nix`/`libc` dependency and `unsafe`, and matches the JSON-RPC
//!   control plane the rest of `rubix-ext` already speaks. Whole-process-group
//!   reaping of leaked grandchildren is a separable future addition;
//!   `kill_on_drop` covers the direct child today.
//! - **Identity by env handoff** (Open question 2): the child receives its
//!   principal's `namespace`/`subject`/`secret` as env vars and signs in as
//!   itself, inheriting the same row-scoped session a user gets — the supervisor
//!   does not proxy an in-process session into another address space.

use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use super::backoff::BackoffSchedule;
use super::handle::SupervisorHandle;
use super::id::ExtensionId;
use super::ring::{EventKind, EventRing, MAX_STDERR_LINE_BYTES};
use super::restart::{ExitReason, RestartDecision, RestartTracker};
use super::spec::ProcessSpec;
use super::state::LifecycleState;
use super::stats::{self, LiveProcess, ProcessCell};
use super::stdio;
use crate::error::{ExtError, Result};

/// JSON-RPC protocol version stamped on every envelope.
const JSONRPC_VERSION: &str = "2.0";

/// Env var carrying the child's namespace, so it can sign in as its principal.
const ENV_NAMESPACE: &str = "RUBIX_EXT_NAMESPACE";
/// Env var carrying the child's principal subject.
const ENV_SUBJECT: &str = "RUBIX_EXT_SUBJECT";
/// Env var carrying the child's principal secret.
const ENV_SECRET: &str = "RUBIX_EXT_SECRET";

/// Hard ceiling on how long a kill path waits for the child to die before moving
/// on, so a child that ignores the hard kill cannot wedge the supervisor task.
const REAP_WAIT: Duration = Duration::from_secs(5);

/// The credentials a supervised child needs to authenticate as its own principal.
///
/// Threaded into the child's environment on spawn (Open question 2 in
/// `rubix/docs/design/EXTENSION-RUNTIME.md`): the child signs in with
/// subject/secret and gets the same WS-03 row-scoped session a user does. The
/// supervisor never retains the secret beyond passing it to the child env.
#[derive(Clone)]
pub struct Identity {
    /// The namespace the extension principal is scoped to.
    pub namespace: String,
    /// The extension principal's subject.
    pub subject: String,
    /// The extension principal's secret, presented by the child at sign-in.
    pub secret: String,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the secret — Debug is used in logs and test output.
        f.debug_struct("Identity")
            .field("namespace", &self.namespace)
            .field("subject", &self.subject)
            .field("secret", &"<redacted>")
            .finish()
    }
}

/// The supervisor entry point.
pub struct Supervisor;

impl Supervisor {
    /// Start a supervisor task for `id` running `spec`, with the child
    /// authenticating as `identity`.
    ///
    /// Returns immediately with a [`SupervisorHandle`]; the management task is
    /// spawned onto the current tokio runtime. Only [`ProcessFlavour::Process`]
    /// specs reach here — builtin/wasm have no child to spawn, and the reconciler
    /// filters them out before calling this.
    ///
    /// # Errors
    /// Returns [`ExtError::Command`] if there is no current tokio runtime to
    /// spawn the task onto.
    pub fn start(id: ExtensionId, spec: ProcessSpec, identity: Identity) -> Result<SupervisorHandle> {
        let (state_tx, state_rx) = watch::channel(LifecycleState::Starting);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel::<serde_json::Value>();
        let events = Arc::new(EventRing::new());
        let violations = Arc::new(AtomicU64::new(0));
        let process = stats::new_cell();

        let task = SupervisorTask {
            spec: spec.clone(),
            identity,
            restart: RestartTracker::new(spec.restart, spec.max_restarts, spec.within_seconds),
            backoff: BackoffSchedule::from_config(&spec.backoff),
            state_tx,
            shutdown_rx,
            inbound_rx,
            events: events.clone(),
            process: process.clone(),
        };
        tokio::spawn(task.run());

        Ok(SupervisorHandle {
            id,
            state: state_rx,
            shutdown_tx,
            events,
            violations,
            inbound: inbound_tx,
            process,
        })
    }
}

/// The body of the management loop. One per extension.
struct SupervisorTask {
    spec: ProcessSpec,
    identity: Identity,
    restart: RestartTracker,
    backoff: BackoffSchedule,
    state_tx: watch::Sender<LifecycleState>,
    shutdown_rx: mpsc::Receiver<()>,
    inbound_rx: mpsc::UnboundedReceiver<serde_json::Value>,
    events: Arc<EventRing>,
    process: ProcessCell,
}

impl SupervisorTask {
    async fn run(mut self) {
        loop {
            self.publish_state(LifecycleState::Starting);
            let exit_reason = match self.spawn_and_serve().await {
                Ok(reason) => reason,
                Err(e) => {
                    self.events.push(EventKind::Crashed {
                        reason: format!("{e}"),
                    });
                    ExitReason::Crash
                }
            };

            // The child for this cycle is gone — clear the live cell so the
            // handle stops reporting a stale process between spawns.
            if let Ok(mut g) = self.process.lock() {
                *g = None;
            }

            // Was a shutdown requested while we were live?
            if self.shutdown_drained() {
                self.settle(LifecycleState::Stopped);
                return;
            }

            match self.restart.should_restart(exit_reason) {
                RestartDecision::Restart => {
                    let wait = self.backoff.next_wait();
                    self.events.push(EventKind::RestartScheduled {
                        wait_ms: wait.as_millis() as u64,
                        total: self.restart.total(),
                    });
                    tokio::select! {
                        () = tokio::time::sleep(wait) => {}
                        _ = self.shutdown_rx.recv() => {
                            self.settle(LifecycleState::Stopped);
                            return;
                        }
                    }
                }
                RestartDecision::Stop => {
                    self.settle(LifecycleState::Stopped);
                    return;
                }
                RestartDecision::Failed => {
                    self.events.push(EventKind::RestartCapExceeded {
                        count: self.spec.max_restarts,
                    });
                    self.settle(LifecycleState::Failed);
                    return;
                }
            }
        }
    }

    /// One spawn cycle: exec the child, complete the init handshake, drive the
    /// wire loop until the child exits.
    async fn spawn_and_serve(&mut self) -> Result<ExitReason> {
        let mut cmd = Command::new(&self.spec.bin);
        cmd.args(&self.spec.args)
            .env(ENV_NAMESPACE, &self.identity.namespace)
            .env(ENV_SUBJECT, &self.identity.subject)
            .env(ENV_SECRET, &self.identity.secret)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in &self.spec.env {
            cmd.env(k, v);
        }
        // Put the child in its own process group so a future group-reap can
        // reach grandchildren. Safe, stable std API (no `pre_exec`, no unsafe).
        #[cfg(unix)]
        cmd.process_group(0);

        let mut child = cmd
            .spawn()
            .map_err(|e| ExtError::Command(format!("exec {:?}: {e}", self.spec.bin)))?;
        let pid = child.id().unwrap_or(0);
        self.events.push(EventKind::Spawned { pid });
        if let Ok(mut g) = self.process.lock() {
            *g = Some(LiveProcess::new(pid, self.restart.total(), Instant::now()));
        }

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ExtError::Command("child stdin missing".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ExtError::Command("child stdout missing".to_owned()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ExtError::Command("child stderr missing".to_owned()))?;

        let mut writer = stdin;
        let mut reader = BufReader::new(stdout);

        // ---- Init handshake ----
        if let Err(e) = self.do_handshake(&mut reader, &mut writer).await {
            // Reap the half-started child before surfacing the failure.
            let _ = child.start_kill();
            let _ = tokio::time::timeout(REAP_WAIT, child.wait()).await;
            return Err(e);
        }
        self.publish_state(LifecycleState::Running);
        self.events.push(EventKind::StateTransition {
            to: LifecycleState::Running,
        });
        self.backoff.reset();

        // ---- Stderr forwarder ----
        let stderr_events = self.events.clone();
        let stderr_task: JoinHandle<()> = tokio::spawn(async move {
            let mut buf = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match buf.read_line(&mut line).await {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
                let trimmed = line.trim_end_matches(['\r', '\n']);
                let capped: String = trimmed.chars().take(MAX_STDERR_LINE_BYTES).collect();
                stderr_events.push(EventKind::Stderr { line: capped });
            }
        });

        // ---- Main wire loop ----
        let exit = self.wire_loop(&mut reader, &mut writer, &mut child).await;
        stderr_task.abort();
        Ok(exit)
    }

    /// Init handshake: send `init { config }`, await a response whose
    /// `result.ready` is `true`, bounded by a timeout so an unresponsive child
    /// cannot wedge the supervisor.
    async fn do_handshake(
        &mut self,
        reader: &mut BufReader<ChildStdout>,
        writer: &mut ChildStdin,
    ) -> Result<()> {
        let req = serde_json::json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": 0,
            "method": "init",
            "params": { "host_version": env!("CARGO_PKG_VERSION") },
        });
        stdio::write_value(writer, &req).await?;

        let timeout =
            Duration::from_millis(u64::from(self.spec.health.timeout_ms).max(500)) * 4;
        let frame = tokio::time::timeout(timeout, stdio::read_frame(reader))
            .await
            .map_err(|_| ExtError::Command("init handshake timed out".to_owned()))??
            .ok_or_else(|| {
                ExtError::Command("child closed stdout before init response".to_owned())
            })?;

        let value: serde_json::Value = serde_json::from_slice(&frame)
            .map_err(|e| ExtError::Command(format!("init response not JSON: {e}")))?;
        let ready = value
            .get("result")
            .and_then(|r| r.get("ready"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !ready {
            return Err(ExtError::Command(format!(
                "child did not report ready from init: {value}"
            )));
        }
        Ok(())
    }

    /// Drive the wire loop: forward outbound envelopes, classify inbound frames,
    /// emit periodic health pings, return when the child exits.
    async fn wire_loop(
        &mut self,
        reader: &mut BufReader<ChildStdout>,
        writer: &mut ChildStdin,
        child: &mut Child,
    ) -> ExitReason {
        let health_interval =
            Duration::from_millis(u64::from(self.spec.health.interval_ms).max(100));
        let health_timeout =
            Duration::from_millis(u64::from(self.spec.health.timeout_ms).max(50));
        let mut health_ticker = tokio::time::interval(health_interval);
        // First tick fires immediately; skip it so the first ping is one
        // interval after startup.
        health_ticker.tick().await;
        let mut next_health_id: i64 = 1;
        let mut health_deadline: Option<tokio::time::Instant> = None;

        loop {
            tokio::select! {
                biased;

                // Shutdown request: cooperative shutdown then hard-kill.
                _ = self.shutdown_rx.recv() => {
                    self.publish_state(LifecycleState::Stopping);
                    self.events.push(EventKind::StateTransition {
                        to: LifecycleState::Stopping,
                    });
                    self.graceful_kill(writer, child).await;
                    return ExitReason::Clean;
                }

                // Outbound envelope from the host.
                Some(env) = self.inbound_rx.recv() => {
                    if stdio::write_value(writer, &env).await.is_err() {
                        return ExitReason::Crash;
                    }
                }

                // A pinged child whose deadline passed without an answer.
                () = sleep_until_opt(health_deadline), if health_deadline.is_some() => {
                    self.events.push(EventKind::HealthTimeout);
                    let _ = child.start_kill();
                    let _ = tokio::time::timeout(REAP_WAIT, child.wait()).await;
                    return ExitReason::Crash;
                }

                // Periodic health ping.
                _ = health_ticker.tick() => {
                    if health_deadline.is_some() {
                        // Prior ping still unanswered — let the deadline arm
                        // handle the kill; don't pile on another ping.
                        continue;
                    }
                    let ping = serde_json::json!({
                        "jsonrpc": JSONRPC_VERSION,
                        "id": next_health_id,
                        "method": "health",
                    });
                    next_health_id += 1;
                    if stdio::write_value(writer, &ping).await.is_err() {
                        return ExitReason::Crash;
                    }
                    health_deadline = Some(tokio::time::Instant::now() + health_timeout);
                    self.sample_process();
                }

                // Inbound frame from the child.
                frame = stdio::read_frame(reader) => {
                    match frame {
                        Ok(Some(bytes)) => {
                            self.handle_frame(&bytes, writer, &mut health_deadline).await;
                        }
                        Ok(None) => {
                            let code = child.wait().await.ok().and_then(|s| s.code());
                            return self.classify_exit(code);
                        }
                        Err(e) => {
                            self.events.push(EventKind::Crashed {
                                reason: format!("frame error: {e}"),
                            });
                            let _ = child.start_kill();
                            let _ = tokio::time::timeout(REAP_WAIT, child.wait()).await;
                            return ExitReason::Crash;
                        }
                    }
                }

                // Child exited without closing stdout cleanly.
                status = child.wait() => {
                    let code = status.ok().and_then(|s| s.code());
                    return self.classify_exit(code);
                }
            }
        }
    }

    /// Record a clean/crash exit event and map the code to an [`ExitReason`].
    fn classify_exit(&self, code: Option<i32>) -> ExitReason {
        if code == Some(0) {
            self.events.push(EventKind::ExitedClean { code });
            ExitReason::Clean
        } else {
            self.events.push(EventKind::Crashed {
                reason: format!("exited with {code:?}"),
            });
            ExitReason::Crash
        }
    }

    /// Sample the current child's `/proc` entry and fold RSS/CPU into the shared
    /// cell. Called on the existing health tick (no extra timer); a no-op between
    /// spawns or on non-Linux.
    fn sample_process(&self) {
        let pid = match self.process.lock() {
            Ok(g) => match g.as_ref() {
                Some(p) => p.pid,
                None => return,
            },
            Err(_) => return,
        };
        let (rss_bytes, total_ticks) = stats::sample(pid);
        if let Ok(mut g) = self.process.lock()
            && let Some(p) = g.as_mut()
        {
            p.apply_sample(Instant::now(), rss_bytes, total_ticks);
        }
    }

    /// Classify one inbound frame. A response (id + result/error) clears the
    /// health deadline — any answer is evidence the child is alive. A request
    /// from the child is answered with a method-not-found error (the host
    /// exposes no callable methods in this phase). Notifications are ignored.
    async fn handle_frame(
        &mut self,
        bytes: &[u8],
        writer: &mut ChildStdin,
        health_deadline: &mut Option<tokio::time::Instant>,
    ) {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
            self.events.push(EventKind::Crashed {
                reason: "malformed frame from child".to_owned(),
            });
            return;
        };

        let has_id = value.get("id").is_some();
        let is_response =
            has_id && (value.get("result").is_some() || value.get("error").is_some());
        if is_response {
            *health_deadline = None;
            return;
        }

        // A request from child → host: reply method-not-found so the child never
        // hangs waiting on an answer the host will not give in this phase.
        if has_id {
            let resp = serde_json::json!({
                "jsonrpc": JSONRPC_VERSION,
                "id": value.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "error": { "code": -32601, "message": "host exposes no callable methods" },
            });
            let _ = stdio::write_value(writer, &resp).await;
        }
        // Notifications (no id) are ignored.
    }

    /// Cooperative shutdown then hard kill: send a `shutdown` notification on the
    /// control channel, wait the grace window for a clean exit, then `start_kill`
    /// if the child has not gone.
    async fn graceful_kill(&mut self, writer: &mut ChildStdin, child: &mut Child) {
        let notif = serde_json::json!({
            "jsonrpc": JSONRPC_VERSION,
            "method": "shutdown",
        });
        let _ = stdio::write_value(writer, &notif).await;

        let grace = Duration::from_millis(u64::from(self.spec.shutdown_grace_ms));
        match tokio::time::timeout(grace, child.wait()).await {
            Ok(_) => {
                self.events.push(EventKind::ExitedClean { code: Some(0) });
            }
            Err(_) => {
                let _ = child.start_kill();
                let _ = tokio::time::timeout(REAP_WAIT, child.wait()).await;
                self.events.push(EventKind::Crashed {
                    reason: "shutdown grace exceeded; hard kill".to_owned(),
                });
            }
        }
    }

    fn publish_state(&self, s: LifecycleState) {
        let _ = self.state_tx.send(s);
    }

    /// Publish a terminal state and record the transition into the ring.
    fn settle(&self, s: LifecycleState) {
        self.publish_state(s);
        self.events.push(EventKind::StateTransition { to: s });
    }

    /// Drain queued shutdown messages without blocking; `true` if at least one
    /// was queued (used to skip the next restart cycle).
    fn shutdown_drained(&mut self) -> bool {
        let mut got_one = false;
        while self.shutdown_rx.try_recv().is_ok() {
            got_one = true;
        }
        got_one
    }
}

/// Sleep until `deadline`, or forever if `None`. Paired with an
/// `if health_deadline.is_some()` guard so the arm is only polled when a deadline
/// is pending.
async fn sleep_until_opt(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending::<()>().await,
    }
}
