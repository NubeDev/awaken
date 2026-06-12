//! Keyexpr-prefix capability grants. A driver's session is confined to the
//! prefixes its manifest declares; the bus enforces these at publish/query
//! time per STACK-DEISGN.md ("each driver gets a scoped zenoh session limited
//! to its granted keyexpr prefixes").

use serde::{Deserialize, Serialize};

use crate::error::DriverError;

/// What a driver may do on a keyexpr prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// Publish `cur` and reply to queries under the prefix.
    Publish,
    /// Subscribe and issue queries under the prefix.
    Subscribe,
    /// Both directions.
    All,
}

impl Access {
    fn allows_publish(self) -> bool {
        matches!(self, Access::Publish | Access::All)
    }

    fn allows_subscribe(self) -> bool {
        matches!(self, Access::Subscribe | Access::All)
    }
}

/// A grant of [`Access`] over a keyexpr prefix such as
/// `nube/hq/ahu-3/**`. The trailing `/**` is implied: the prefix matches
/// itself and any key beneath it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub prefix: String,
    pub access: Access,
}

impl Capability {
    /// Validate the prefix shape: non-empty, no leading/trailing slash, no
    /// wildcards (the prefix *is* the wildcard root, expressed literally).
    pub fn validate(&self) -> Result<(), DriverError> {
        let p = &self.prefix;
        if p.is_empty() {
            return Err(DriverError::InvalidPrefix(p.clone(), "empty"));
        }
        if p.starts_with('/') || p.ends_with('/') {
            return Err(DriverError::InvalidPrefix(
                p.clone(),
                "leading/trailing slash",
            ));
        }
        if p.contains('*') {
            return Err(DriverError::InvalidPrefix(
                p.clone(),
                "wildcards not allowed",
            ));
        }
        Ok(())
    }

    /// True if `key` falls under this prefix: equal to it, or beneath a `/`.
    pub fn covers(&self, key: &str) -> bool {
        key == self.prefix
            || key
                .strip_prefix(&self.prefix)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    fn grants(&self, key: &str, action: Access) -> bool {
        if !self.covers(key) {
            return false;
        }
        match action {
            Access::Publish => self.access.allows_publish(),
            Access::Subscribe => self.access.allows_subscribe(),
            Access::All => self.access.allows_publish() && self.access.allows_subscribe(),
        }
    }
}

/// The set of grants held by one driver session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub grants: Vec<Capability>,
}

impl CapabilitySet {
    pub fn validate(&self) -> Result<(), DriverError> {
        self.grants.iter().try_for_each(Capability::validate)
    }

    /// True if any grant permits `action` on `key`.
    pub fn allows(&self, key: &str, action: Access) -> bool {
        self.grants.iter().any(|g| g.grants(key, action))
    }

    /// Authorize a publish, or return a [`DriverError::Denied`] naming `driver`.
    pub fn authorize_publish(&self, driver: &str, key: &str) -> Result<(), DriverError> {
        self.guard(driver, key, "publish", Access::Publish)
    }

    /// Authorize a subscribe/query, or return a [`DriverError::Denied`].
    pub fn authorize_subscribe(&self, driver: &str, key: &str) -> Result<(), DriverError> {
        self.guard(driver, key, "subscribe", Access::Subscribe)
    }

    fn guard(
        &self,
        driver: &str,
        key: &str,
        action: &'static str,
        access: Access,
    ) -> Result<(), DriverError> {
        if self.allows(key, access) {
            Ok(())
        } else {
            Err(DriverError::Denied {
                driver: driver.to_string(),
                action,
                key: key.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(prefix: &str, access: Access) -> Capability {
        Capability {
            prefix: prefix.to_string(),
            access,
        }
    }

    #[test]
    fn covers_self_and_descendants_but_not_siblings() {
        let c = cap("nube/hq/ahu-3", Access::All);
        assert!(c.covers("nube/hq/ahu-3"));
        assert!(c.covers("nube/hq/ahu-3/fan/cur"));
        // prefix-string match that is not a path boundary must not pass.
        assert!(!c.covers("nube/hq/ahu-30/fan"));
        assert!(!c.covers("nube/hq/ahu-2"));
    }

    #[test]
    fn access_gates_direction() {
        let set = CapabilitySet {
            grants: vec![cap("nube/hq/ahu-3", Access::Publish)],
        };
        assert!(set.allows("nube/hq/ahu-3/fan/cur", Access::Publish));
        assert!(!set.allows("nube/hq/ahu-3/fan/write", Access::Subscribe));
    }

    #[test]
    fn authorize_names_driver_on_denial() {
        let set = CapabilitySet {
            grants: vec![cap("nube/hq/ahu-3", Access::All)],
        };
        let err = set
            .authorize_publish("bacnet", "nube/hq/ahu-9/fan/cur")
            .unwrap_err();
        assert_eq!(
            err,
            DriverError::Denied {
                driver: "bacnet".into(),
                action: "publish",
                key: "nube/hq/ahu-9/fan/cur".into(),
            }
        );
    }

    #[test]
    fn validate_rejects_wildcards_and_edge_slashes() {
        assert!(cap("nube/hq/**", Access::All).validate().is_err());
        assert!(cap("/nube/hq", Access::All).validate().is_err());
        assert!(cap("nube/hq/", Access::All).validate().is_err());
        assert!(cap("nube/hq/ahu-3", Access::All).validate().is_ok());
    }
}
