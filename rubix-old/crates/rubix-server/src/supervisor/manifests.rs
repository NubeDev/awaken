//! Load driver manifests from the `RUBIX_DRIVERS` JSON file. The file is a JSON
//! array of [`DriverManifest`]s the supervisor spawns at boot. Absent path or
//! file means "no drivers" — a valid, common configuration (cloud nodes run no
//! local drivers). A present-but-malformed file fails closed: a typo in driver
//! config must not silently disable supervision.

use std::path::Path;

use rubix_driver::DriverManifest;

use super::SupervisorError;

/// Read and parse the manifests at `path`. Returns an empty vec if the file
/// does not exist; errors only on read or decode failure.
pub fn load_manifests(path: &Path) -> Result<Vec<DriverManifest>, SupervisorError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)
        .map_err(|e| SupervisorError::Manifests(format!("read {path:?}: {e}")))?;
    let manifests: Vec<DriverManifest> = serde_json::from_slice(&bytes)
        .map_err(|e| SupervisorError::Manifests(format!("parse {path:?}: {e}")))?;
    Ok(manifests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_no_drivers() {
        let path = Path::new("/no/such/rubix-drivers.json");
        assert!(load_manifests(path).expect("missing ok").is_empty());
    }

    #[test]
    fn malformed_file_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("drivers.json");
        std::fs::write(&path, b"{ not json }").expect("write");
        assert!(load_manifests(&path).is_err());
    }

    #[test]
    fn valid_array_parses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("drivers.json");
        let json = serde_json::json!([{
            "identity": {
                "name": "bacnet", "protocol": "bacnet-ip", "version": "0.1.0",
                "launch": {"command": "rubix-driver-bacnet", "args": []}
            },
            "capabilities": {"grants": [{"prefix": "nube/hq/ahu-3", "access": "all"}]}
        }]);
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).expect("write");
        let manifests = load_manifests(&path).expect("parse");
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].identity.name, "bacnet");
    }
}
