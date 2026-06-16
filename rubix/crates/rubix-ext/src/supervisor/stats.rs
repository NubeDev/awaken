//! [`ProcessStats`] + live-process bookkeeping + `/proc` sampling.
//!
//! Ported from `starter-ext-supervisor::proc_stats` and `starter-ext-spi::process`
//! (`rubix/docs/design/EXTENSION-RUNTIME.md`, "Status & metrics projection"). The
//! supervisor stores the current child's pid (plus the data to build a
//! [`ProcessStats`]) in a [`ProcessCell`] shared with the
//! [`SupervisorHandle`](crate::supervisor::SupervisorHandle); the cell is filled
//! on spawn and cleared on exit, so the handle reads it lock-free of the task.
//!
//! RSS / CPU are **sampled on the supervisor's existing health tick** — no extra
//! collector thread — by reading `/proc/<pid>/stat` + `/proc/<pid>/statm` on
//! Linux. Other platforms (and any read/parse failure) leave the gauges `None`.
//! The parsers are free functions so they can be unit-tested against captured
//! `/proc` text without a live process.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};

/// `USER_HZ` — scheduler ticks per second the kernel reports `utime`/`stime` in.
/// 100 on every mainstream Linux build; assumed rather than pulling in `libc`
/// for `sysconf`. A wrong value only scales the best-effort `cpu_pct` gauge.
const USER_HZ: f64 = 100.0;

/// Page size in bytes, to turn `/proc/<pid>/statm` resident *pages* into bytes.
const PAGE_SIZE: u64 = 4096;

/// Live process statistics for a process-flavour extension's current child.
/// Returned by [`SupervisorHandle::process_stats`](crate::supervisor::SupervisorHandle::process_stats)
/// and served by `GET /extensions/<id>/process`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessStats {
    /// OS process id of the current child.
    pub pid: u32,
    /// Wall-clock time the current child was spawned. Resets on restart.
    pub started_at: SystemTime,
    /// How long the current child has been alive (since `started_at`).
    pub uptime: Duration,
    /// Resident set size in bytes, sampled on the last health tick. `None` on
    /// platforms without `/proc`, or before the first sample.
    pub rss_bytes: Option<u64>,
    /// CPU usage as a percentage of one core, averaged over the interval between
    /// the last two health samples. `None` until two samples exist.
    pub cpu_pct: Option<f32>,
    /// Times the supervisor has restarted this extension over the process's
    /// lifetime (carried across child respawns).
    pub restarts: u64,
}

/// State of the current child, shared between the supervisor task (writer) and
/// the [`SupervisorHandle`](crate::supervisor::SupervisorHandle) (reader).
#[derive(Debug, Clone)]
pub(crate) struct LiveProcess {
    pub pid: u32,
    pub started_at: SystemTime,
    pub started_instant: Instant,
    pub restarts: u64,
    pub rss_bytes: Option<u64>,
    pub cpu_pct: Option<f32>,
    pub last_cpu_ticks: Option<u64>,
    pub last_sample_at: Option<Instant>,
}

impl LiveProcess {
    /// A freshly-spawned child with no samples yet.
    pub(crate) fn new(pid: u32, restarts: u64, now: Instant) -> Self {
        Self {
            pid,
            started_at: SystemTime::now(),
            started_instant: now,
            restarts,
            rss_bytes: None,
            cpu_pct: None,
            last_cpu_ticks: None,
            last_sample_at: None,
        }
    }

    /// Project the bookkeeping into the wire [`ProcessStats`] shape. `now` is the
    /// read instant used for `uptime`.
    pub(crate) fn to_stats(&self, now: Instant) -> ProcessStats {
        ProcessStats {
            pid: self.pid,
            started_at: self.started_at,
            uptime: now.saturating_duration_since(self.started_instant),
            rss_bytes: self.rss_bytes,
            cpu_pct: self.cpu_pct,
            restarts: self.restarts,
        }
    }

    /// Fold a fresh `/proc` reading into the running CPU/RSS gauges.
    /// `total_ticks` is `utime + stime`; `cpu_pct` is the delta against the
    /// previous sample over the elapsed window.
    pub(crate) fn apply_sample(
        &mut self,
        now: Instant,
        rss_bytes: Option<u64>,
        total_ticks: Option<u64>,
    ) {
        if rss_bytes.is_some() {
            self.rss_bytes = rss_bytes;
        }
        if let Some(ticks) = total_ticks {
            if let (Some(prev_ticks), Some(prev_at)) = (self.last_cpu_ticks, self.last_sample_at) {
                let elapsed = now.saturating_duration_since(prev_at).as_secs_f64();
                let dticks = ticks.saturating_sub(prev_ticks) as f64;
                if elapsed > 0.0 {
                    let pct = (dticks / USER_HZ) / elapsed * 100.0;
                    self.cpu_pct = Some(pct as f32);
                }
            }
            self.last_cpu_ticks = Some(ticks);
            self.last_sample_at = Some(now);
        }
    }
}

/// Shared cell holding the current child's [`LiveProcess`], or `None` when no
/// child is running.
pub(crate) type ProcessCell = Arc<Mutex<Option<LiveProcess>>>;

/// A fresh, empty [`ProcessCell`].
pub(crate) fn new_cell() -> ProcessCell {
    Arc::new(Mutex::new(None))
}

/// Read `/proc/<pid>/stat` + `/statm` and return `(rss_bytes, total_cpu_ticks)`.
/// Linux only; every other platform — and any read/parse failure — yields
/// `(None, None)`.
#[cfg(target_os = "linux")]
pub(crate) fn sample(pid: u32) -> (Option<u64>, Option<u64>) {
    let ticks = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| parse_stat_total_ticks(&s));
    let rss = std::fs::read_to_string(format!("/proc/{pid}/statm"))
        .ok()
        .and_then(|s| parse_statm_rss_bytes(&s));
    (rss, ticks)
}

/// Non-Linux platforms have no `/proc`; stats stay `None`.
#[cfg(not(target_os = "linux"))]
pub(crate) fn sample(_pid: u32) -> (Option<u64>, Option<u64>) {
    (None, None)
}

/// Parse `utime + stime` (fields 14 + 15, 1-indexed) from `/proc/<pid>/stat`, in
/// clock ticks. Field 2 (`comm`) is parenthesised and may contain spaces and
/// `)`, so we split *after the last* `')'`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_stat_total_ticks(contents: &str) -> Option<u64> {
    let close = contents.rfind(')')?;
    let tail = contents.get(close + 1..)?;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime.saturating_add(stime))
}

/// Parse the resident-set size from `/proc/<pid>/statm` and return it in bytes —
/// the second whitespace-delimited field is the resident page count.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_statm_rss_bytes(contents: &str) -> Option<u64> {
    let resident_pages: u64 = contents.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages.saturating_mul(PAGE_SIZE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_with_simple_comm() {
        let line = "1234 (myproc) S 1 1234 1234 0 -1 4194560 100 0 0 0 120 30 0 0 20 0 1 0 99 0";
        assert_eq!(parse_stat_total_ticks(line), Some(150));
    }

    #[test]
    fn parses_stat_with_spaces_and_parens_in_comm() {
        let line = "77 (we ird )name) S 1 77 77 0 -1 0 5 0 0 0 7 3 0 0 20 0 1 0 0 0";
        assert_eq!(parse_stat_total_ticks(line), Some(10));
    }

    #[test]
    fn stat_parse_rejects_garbage() {
        assert_eq!(parse_stat_total_ticks("no parens here"), None);
        assert_eq!(parse_stat_total_ticks("123 (c) S 1"), None);
    }

    #[test]
    fn parses_statm_rss() {
        let line = "2048 512 128 1 0 256 0";
        assert_eq!(parse_statm_rss_bytes(line), Some(512 * PAGE_SIZE));
    }

    #[test]
    fn statm_parse_rejects_garbage() {
        assert_eq!(parse_statm_rss_bytes("onlyonefield"), None);
        assert_eq!(parse_statm_rss_bytes(""), None);
    }

    #[test]
    fn cpu_pct_computed_from_second_sample() {
        let t0 = Instant::now();
        let mut lp = LiveProcess::new(10, 0, t0);
        lp.apply_sample(t0, Some(4096), Some(100));
        assert_eq!(lp.cpu_pct, None);
        assert_eq!(lp.rss_bytes, Some(4096));
        let t1 = t0 + Duration::from_secs(1);
        lp.apply_sample(t1, Some(8192), Some(150));
        assert_eq!(lp.rss_bytes, Some(8192));
        let pct = lp.cpu_pct.expect("cpu_pct after two samples");
        assert!((pct - 50.0).abs() < 0.01, "expected ~50%, got {pct}");
    }

    #[test]
    fn to_stats_reports_uptime() {
        let t0 = Instant::now();
        let lp = LiveProcess::new(42, 3, t0);
        let stats = lp.to_stats(t0 + Duration::from_secs(5));
        assert_eq!(stats.pid, 42);
        assert_eq!(stats.restarts, 3);
        assert_eq!(stats.uptime, Duration::from_secs(5));
    }
}
