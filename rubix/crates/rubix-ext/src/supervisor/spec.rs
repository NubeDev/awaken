//! [`ProcessSpec`] — how to spawn and supervise one extension's child.
//!
//! rubix has no `block.yaml` manifest (`rubix/docs/design/EXTENSION-RUNTIME.md`,
//! Open question 3): the spawn parameters live on the extension's **gated config
//! record** — the one `rubix-ext` `register` / `configure` already writes — under
//! a `runtime` field. This is the rubix-native equivalent of starter's
//! `manifest.runtime` + `manifest.supervision` blocks, collapsed into one
//! deserializable struct so the bridge can read it straight off the config record
//! and the boot reconciler can rebuild a supervisor from it.
//!
//! Every field past `bin` has a sane default, so a minimal config record needs
//! only `{ "runtime": { "bin": "..." } }` to be supervisable.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::backoff::Backoff;
use super::flavour::ProcessFlavour;
use super::restart::RestartPolicy;

/// Health-probe cadence for a supervised child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthConfig {
    /// How often the supervisor pings the child with a `health` request, ms.
    pub interval_ms: u32,
    /// How long a ping may go unanswered before the child is treated as crashed,
    /// ms.
    pub timeout_ms: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            interval_ms: 5_000,
            timeout_ms: 2_000,
        }
    }
}

/// Everything the supervisor needs to spawn and supervise one extension child.
///
/// Deserialized from the `runtime` field of the extension's gated config record.
/// The `bin` path is resolved relative to the host's working directory (or
/// absolute); `args`/`env` are passed through to the child, on top of the
/// identity env the supervisor always injects (subject/secret/namespace) so the
/// child can sign in as its own principal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessSpec {
    /// Path to the child binary to exec.
    pub bin: PathBuf,
    /// Extra command-line arguments passed to the child.
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment variables (key, value) passed to the child, on top of
    /// the identity env the supervisor injects.
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// Packaging flavour. Only [`ProcessFlavour::Process`] is supervised; the
    /// reconciler skips builtin/wasm (they have no child to spawn).
    #[serde(default)]
    pub flavour: ProcessFlavour,
    /// Restart policy after the child exits.
    #[serde(default)]
    pub restart: RestartPolicy,
    /// Max restarts within [`Self::within_seconds`] before settling on `Failed`.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
    /// Sliding window, in seconds, the restart intensity cap is measured over.
    #[serde(default = "default_within_seconds")]
    pub within_seconds: u32,
    /// Exponential backoff between restarts.
    #[serde(default)]
    pub backoff: Backoff,
    /// Health-probe cadence.
    #[serde(default)]
    pub health: HealthConfig,
    /// Grace window after a cooperative `shutdown` before the supervisor escalates
    /// to a hard kill, ms.
    #[serde(default = "default_shutdown_grace_ms")]
    pub shutdown_grace_ms: u32,
}

fn default_max_restarts() -> u32 {
    5
}

fn default_within_seconds() -> u32 {
    60
}

fn default_shutdown_grace_ms() -> u32 {
    5_000
}

impl ProcessSpec {
    /// A minimal spec for `bin` with every supervision default.
    #[must_use]
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: bin.into(),
            args: Vec::new(),
            env: Vec::new(),
            flavour: ProcessFlavour::default(),
            restart: RestartPolicy::default(),
            max_restarts: default_max_restarts(),
            within_seconds: default_within_seconds(),
            backoff: Backoff::default(),
            health: HealthConfig::default(),
            shutdown_grace_ms: default_shutdown_grace_ms(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_runtime_record_deserialises() {
        let spec: ProcessSpec = serde_json::from_value(serde_json::json!({
            "bin": "/usr/bin/my-ext",
        }))
        .expect("a bare bin is a valid spec");
        assert_eq!(spec.bin, PathBuf::from("/usr/bin/my-ext"));
        assert_eq!(spec.flavour, ProcessFlavour::Process);
        assert_eq!(spec.restart, RestartPolicy::OnCrash);
        assert_eq!(spec.max_restarts, 5);
    }

    #[test]
    fn full_runtime_record_round_trips() {
        let spec = ProcessSpec {
            bin: PathBuf::from("./ext"),
            args: vec!["--serve".into()],
            env: vec![("LOG".into(), "debug".into())],
            flavour: ProcessFlavour::Process,
            restart: RestartPolicy::Always,
            max_restarts: 3,
            within_seconds: 30,
            backoff: Backoff::default(),
            health: HealthConfig::default(),
            shutdown_grace_ms: 1_000,
        };
        let j = serde_json::to_value(&spec).unwrap();
        let back: ProcessSpec = serde_json::from_value(j).unwrap();
        assert_eq!(back, spec);
    }
}
