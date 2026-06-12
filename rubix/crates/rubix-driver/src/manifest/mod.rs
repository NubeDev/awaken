//! Driver manifest: the contract a driver declares to the supervisor —
//! identity, contributed point types, granted capabilities, and config schema.

mod capability;
mod identity;
mod point_type;

pub use capability::{Access, Capability, CapabilitySet};
pub use identity::{Identity, Launch};
pub use point_type::{PinDirection, PointType};

use serde::{Deserialize, Serialize};

use crate::error::DriverError;

/// The full manifest a driver ships and the supervisor validates before spawn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverManifest {
    pub identity: Identity,
    /// Point types contributed to the node palette.
    #[serde(default)]
    pub point_types: Vec<PointType>,
    /// Keyexpr prefixes the driver's scoped session is confined to.
    pub capabilities: CapabilitySet,
    /// Opaque, driver-specific configuration validated by the driver itself.
    #[serde(default)]
    pub config: serde_json::Value,
}

impl DriverManifest {
    /// Validate identity completeness and every capability prefix. A manifest
    /// with no capabilities is rejected: a driver that can touch no keyexpr is
    /// inert and almost certainly a misconfiguration.
    pub fn validate(&self) -> Result<(), DriverError> {
        if self.identity.name.is_empty() {
            return Err(DriverError::InvalidManifest(
                "identity.name is empty".into(),
            ));
        }
        if self.identity.protocol.is_empty() {
            return Err(DriverError::InvalidManifest(
                "identity.protocol is empty".into(),
            ));
        }
        if self.identity.launch.command.is_empty() {
            return Err(DriverError::InvalidManifest(
                "identity.launch.command is empty".into(),
            ));
        }
        if self.capabilities.grants.is_empty() {
            return Err(DriverError::InvalidManifest(
                "no capabilities granted".into(),
            ));
        }
        self.capabilities.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> DriverManifest {
        DriverManifest {
            identity: Identity {
                name: "bacnet".into(),
                protocol: "bacnet-ip".into(),
                version: "0.1.0".into(),
                launch: Launch {
                    command: "rubix-driver-bacnet".into(),
                    args: vec![],
                },
            },
            point_types: vec![],
            capabilities: CapabilitySet {
                grants: vec![Capability {
                    prefix: "nube/hq/ahu-3".into(),
                    access: Access::All,
                }],
            },
            config: serde_json::Value::Null,
        }
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(manifest().validate().is_ok());
    }

    #[test]
    fn driver_with_no_capabilities_is_rejected() {
        let mut m = manifest();
        m.capabilities.grants.clear();
        assert!(m.validate().is_err());
    }

    #[test]
    fn roundtrips_through_json() {
        let m = manifest();
        let json = serde_json::to_string(&m).unwrap();
        let back: DriverManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
