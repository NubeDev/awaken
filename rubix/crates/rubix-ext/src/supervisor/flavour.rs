//! [`ProcessFlavour`] — does this extension have a host-visible child process?
//!
//! Ported from `starter-ext-spi` (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! "What rubix means by an extension that runs"). rubix only makes the
//! **process** flavour real on day one; `builtin` collapses to "no process" (the
//! host does the work directly under the extension principal) and `wasm` is
//! deferred. The flavour is the single bit the process/metrics projections key
//! the "report stats vs report null" decision on, so it lives next to the stats
//! shapes rather than being re-derived per endpoint.

use serde::{Deserialize, Serialize};

/// Which packaging flavour an extension is, projected to the one bit the runtime
/// cares about: is there a host-visible child process whose stats can be sampled?
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessFlavour {
    /// A child process spawned and supervised by the runtime; reports stats.
    #[default]
    Process,
    /// Runs inside the host under the extension principal; no host-visible
    /// child. The host pid is never reported. Reports `null` stats.
    Builtin,
    /// WASI component instantiated in-process; no child. Deferred — reports
    /// `null` stats today.
    Wasm,
}

impl ProcessFlavour {
    /// Whether this flavour has a host-visible child process whose
    /// [`ProcessStats`](crate::supervisor::ProcessStats) can be sampled. Only
    /// [`Process`](ProcessFlavour::Process) does; builtin/wasm report `null` and
    /// the UI hides the tab.
    #[must_use]
    pub const fn reports_process_stats(self) -> bool {
        matches!(self, Self::Process)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flavour_round_trips_snake_case() {
        assert_eq!(
            serde_json::to_string(&ProcessFlavour::Process).unwrap(),
            "\"process\""
        );
        let f: ProcessFlavour = serde_json::from_str("\"wasm\"").unwrap();
        assert_eq!(f, ProcessFlavour::Wasm);
    }

    #[test]
    fn only_process_reports_stats() {
        assert!(ProcessFlavour::Process.reports_process_stats());
        assert!(!ProcessFlavour::Builtin.reports_process_stats());
        assert!(!ProcessFlavour::Wasm.reports_process_stats());
    }

    #[test]
    fn process_is_the_default() {
        assert_eq!(ProcessFlavour::default(), ProcessFlavour::Process);
    }
}
