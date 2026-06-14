//! Read the driver contract from the environment the supervisor injects:
//! name, granted capabilities, and the sim's own config blob. Fails closed —
//! a missing name or unparseable caps aborts before touching the bus.

use std::time::Duration;

use rubix_driver::{CapabilitySet, ENV_DRIVER_CAPS, ENV_DRIVER_CONFIG, ENV_DRIVER_NAME};
use serde::Deserialize;

/// The sim's resolved runtime configuration.
pub struct SimConfig {
    /// Driver name (liveliness-token key, log identity).
    pub name: String,
    /// Granted capabilities the sim confines its publishes to.
    pub caps: CapabilitySet,
    /// Point keyexpr prefix to publish `cur` on, e.g. `nube/hq/ahu-3/temp`.
    pub point: String,
    /// Seconds between published samples.
    pub period: Duration,
    /// Value the simulated sensor oscillates around.
    pub baseline: f64,
    /// Peak deviation from `baseline`.
    pub amplitude: f64,
    /// Bound on the outbound `cur` buffer. Under publish backpressure the oldest
    /// queued samples drop (latest-wins) and are counted — never silently lost.
    pub cur_capacity: usize,
}

/// The driver-specific `config` blob, decoded from `RUBIX_DRIVER_CONFIG`.
#[derive(Debug, Deserialize)]
struct ConfigBlob {
    /// Point keyexpr prefix to publish on (required).
    point: String,
    #[serde(default = "default_period_secs")]
    period_secs: u64,
    #[serde(default = "default_baseline")]
    baseline: f64,
    #[serde(default = "default_amplitude")]
    amplitude: f64,
    #[serde(default = "default_cur_capacity")]
    cur_capacity: usize,
}

fn default_period_secs() -> u64 {
    5
}
fn default_baseline() -> f64 {
    21.0
}
fn default_amplitude() -> f64 {
    2.0
}
fn default_cur_capacity() -> usize {
    64
}

impl SimConfig {
    /// Resolve from the injected environment. Returns an error describing the
    /// first missing/invalid piece.
    pub fn from_env() -> anyhow::Result<Self> {
        let name = std::env::var(ENV_DRIVER_NAME)
            .map_err(|_| anyhow::anyhow!("{ENV_DRIVER_NAME} is required"))?;
        let caps_json = std::env::var(ENV_DRIVER_CAPS)
            .map_err(|_| anyhow::anyhow!("{ENV_DRIVER_CAPS} is required"))?;
        let caps: CapabilitySet = serde_json::from_str(&caps_json)
            .map_err(|e| anyhow::anyhow!("{ENV_DRIVER_CAPS} is not a CapabilitySet: {e}"))?;
        caps.validate()
            .map_err(|e| anyhow::anyhow!("invalid capabilities: {e}"))?;

        let config_json = std::env::var(ENV_DRIVER_CONFIG).unwrap_or_else(|_| "null".into());
        let blob: ConfigBlob = serde_json::from_str(&config_json)
            .map_err(|e| anyhow::anyhow!("{ENV_DRIVER_CONFIG} is not sim config: {e}"))?;
        if blob.period_secs == 0 {
            anyhow::bail!("config.period_secs must be >= 1");
        }

        Ok(SimConfig {
            name,
            caps,
            point: blob.point,
            period: Duration::from_secs(blob.period_secs),
            baseline: blob.baseline,
            amplitude: blob.amplitude,
            cur_capacity: blob.cur_capacity,
        })
    }
}
