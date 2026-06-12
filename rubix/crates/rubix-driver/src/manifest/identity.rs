//! Driver identity: stable name, protocol, version, and launch command.

use serde::{Deserialize, Serialize};

/// How the supervisor launches the driver process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launch {
    /// Executable path or name resolved on `PATH`.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Stable identity of a driver build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// Stable slug, unique per node, e.g. `bacnet`. Also the liveliness-token
    /// and supervision key.
    pub name: String,
    /// Field protocol the driver speaks, e.g. `bacnet-ip`, `modbus-tcp`.
    pub protocol: String,
    /// Semver of the driver build.
    pub version: String,
    pub launch: Launch,
}
