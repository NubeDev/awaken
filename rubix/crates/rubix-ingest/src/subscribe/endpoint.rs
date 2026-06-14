//! The Zenoh peer-session configuration an ingest subscriber opens on.
//!
//! Ingestion runs over a Zenoh peer session (`rubix/docs/SCOPE.md`, "Ingestion
//! and pre-processing": sources publish to Zenoh; the platform consumes in
//! flight). This file owns the small set of session knobs the platform sets —
//! the listen/connect endpoints and whether multicast scouting is on — so the
//! rest of the crate composes a `zenoh::Config` without reaching into Zenoh's
//! JSON5 surface. Defaults match a self-contained edge node: scout for peers on
//! the local fabric, bind nothing explicitly.

use zenoh::Config;

use crate::error::{IngestError, Result};

/// How an ingest subscriber's Zenoh peer session reaches the data fabric.
///
/// `listen` endpoints accept inbound peer links; `connect` endpoints dial known
/// peers directly. `multicast_scouting` toggles UDP multicast peer discovery —
/// left on for a LAN edge, turned off when peers are wired explicitly (e.g. a
/// loopback test or a locked-down deployment).
#[derive(Debug, Clone)]
pub struct ZenohEndpoint {
    /// Endpoints the session listens on for inbound peer links (e.g.
    /// `tcp/127.0.0.1:7447`).
    pub listen: Vec<String>,
    /// Endpoints the session dials to reach known peers.
    pub connect: Vec<String>,
    /// Whether UDP multicast peer scouting is enabled.
    pub multicast_scouting: bool,
}

impl Default for ZenohEndpoint {
    fn default() -> Self {
        Self {
            listen: Vec::new(),
            connect: Vec::new(),
            multicast_scouting: true,
        }
    }
}

impl ZenohEndpoint {
    /// Translate the endpoint into a Zenoh peer `Config`.
    ///
    /// # Errors
    /// Returns [`IngestError::Session`] if any endpoint string is not accepted by
    /// Zenoh's configuration parser.
    pub fn to_config(&self) -> Result<Config> {
        let mut config = Config::default();
        set_json(&mut config, "mode", "\"peer\"")?;
        set_json(
            &mut config,
            "scouting/multicast/enabled",
            if self.multicast_scouting { "true" } else { "false" },
        )?;
        if !self.listen.is_empty() {
            set_json(&mut config, "listen/endpoints", &json_array(&self.listen))?;
        }
        if !self.connect.is_empty() {
            set_json(&mut config, "connect/endpoints", &json_array(&self.connect))?;
        }
        Ok(config)
    }
}

/// Insert one JSON5 fragment into the config, mapping the failure to a domain
/// error instead of unwrapping.
fn set_json(config: &mut Config, key: &str, value: &str) -> Result<()> {
    config
        .insert_json5(key, value)
        .map_err(|e| IngestError::Session(format!("config {key}: {e}")))
}

/// Render a list of endpoint strings as a JSON array literal.
fn json_array(values: &[String]) -> String {
    let quoted: Vec<String> = values.iter().map(|v| format!("\"{v}\"")).collect();
    format!("[{}]", quoted.join(","))
}

#[cfg(test)]
mod tests {
    use super::ZenohEndpoint;

    #[test]
    fn default_scouts_and_binds_nothing() {
        let endpoint = ZenohEndpoint::default();
        assert!(endpoint.multicast_scouting);
        assert!(endpoint.listen.is_empty());
        assert!(endpoint.connect.is_empty());
        endpoint.to_config().expect("default config is valid");
    }

    #[test]
    fn explicit_endpoints_build_a_config() {
        let endpoint = ZenohEndpoint {
            listen: vec!["tcp/127.0.0.1:0".to_owned()],
            connect: vec!["tcp/127.0.0.1:7447".to_owned()],
            multicast_scouting: false,
        };
        endpoint.to_config().expect("explicit config is valid");
    }
}
