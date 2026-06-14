//! The registry of known capabilities — the fail-closed allow-set.
//!
//! Every capability the platform recognises is enumerated by
//! [`Capability::ALL`](crate::capability::Capability::ALL). This module exposes
//! the registry as a membership question so the check path can reject an
//! unregistered capability before it ever consults a grant
//! (`rubix/docs/SCOPE.md`, "Two authz layers"; the layer must fail closed).

use super::kind::Capability;

/// Whether `raw` names a capability the platform knows about.
///
/// An unknown string is not a capability the gate can ever allow, so it denies
/// before any grant lookup. Known capabilities are exactly the variants of
/// [`Capability`].
#[must_use]
pub fn is_registered(raw: &str) -> bool {
    Capability::parse(raw).is_some()
}

#[cfg(test)]
mod tests {
    use super::is_registered;
    use crate::capability::Capability;

    #[test]
    fn known_capabilities_are_registered() {
        for capability in Capability::ALL {
            assert!(is_registered(capability.as_str()));
        }
    }

    #[test]
    fn an_unknown_capability_is_not_registered() {
        assert!(!is_registered("forge-grant"));
        assert!(!is_registered(""));
    }
}
