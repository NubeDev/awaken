//! Integration: the capability layer fails closed.
//!
//! `rubix/docs/SCOPE.md` ("Two authz layers") requires the app-enforced layer to
//! deny by default: an unregistered capability is never allowed, and a principal
//! with no grants is denied every capability. The default state of a fresh
//! principal is "no authority".

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, check_capability, is_registered};

use gate::open::{NS, open_gate_store};

#[test]
fn an_unknown_capability_is_never_registered() {
    assert!(!is_registered("forge-admin"));
    assert!(!is_registered("DATASOURCE-REGISTER"));
    assert!(!is_registered(""));
}

#[test]
fn every_known_capability_is_registered() {
    for capability in Capability::ALL {
        assert!(is_registered(capability.as_str()));
    }
}

#[tokio::test]
async fn a_fresh_principal_is_denied_every_capability() {
    let handle = open_gate_store("fail_closed_default").await;
    let principal = Principal::new(
        Id::from_raw("nobody"),
        NS,
        PrincipalKind::User,
        Role::Viewer,
    );

    for capability in Capability::ALL {
        let allowed = check_capability(handle.raw(), &principal, capability)
            .await
            .expect("check");
        assert!(
            !allowed,
            "{} must be denied with no grant",
            capability.as_str()
        );
    }
}
