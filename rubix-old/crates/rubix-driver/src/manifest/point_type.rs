//! Point types a driver contributes to the node — the pin schema shown in the
//! flow editor's node palette and used to validate driver-created points.

use serde::{Deserialize, Serialize};

/// Direction of a contributed pin relative to the field device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinDirection {
    /// Field → platform (sensor reading).
    Input,
    /// Platform → field (commandable output).
    Output,
}

/// One point type the driver can instantiate, e.g. BACnet `analog-value`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointType {
    /// Stable identifier within the driver, e.g. `analog-input`.
    pub id: String,
    pub display_name: String,
    pub direction: PinDirection,
    /// Engineering unit, when fixed by the protocol (`°C`, `kWh`); `None` if
    /// configured per instance.
    #[serde(default)]
    pub unit: Option<String>,
    /// Haystack marker tags applied to points of this type (`sensor`, `temp`).
    #[serde(default)]
    pub tags: Vec<String>,
}
