//! Integration: `check_capability` allows a held grant and denies a missing one.
//!
//! Exercises the app-enforced authz layer against a live SurrealDB
//! (`rubix/STACK-DEISGN.md`, kv-mem for tests). A principal that holds a grant
//! may perform the action; the same principal is denied a capability it was not
//! granted.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, check_capability, create_grant};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn operator(subject: &str) -> Principal {
    Principal::new(Id::from_raw(subject), NS, PrincipalKind::User, Role::Operator)
}

#[tokio::test]
async fn a_held_grant_is_allowed() {
    let handle = open_gate_store("check_allow").await;
    let grantor = admin();
    let grantee = operator("alice");

    create_grant(handle.raw(), &grantor, &grantee, Capability::RuleInvoke)
        .await
        .expect("create grant");

    let allowed = check_capability(handle.raw(), &grantee, Capability::RuleInvoke)
        .await
        .expect("check");
    assert!(allowed, "a held grant must be allowed");
}

#[tokio::test]
async fn a_missing_grant_is_denied() {
    let handle = open_gate_store("check_deny").await;
    let grantee = operator("alice");

    let allowed = check_capability(handle.raw(), &grantee, Capability::RuleInvoke)
        .await
        .expect("check");
    assert!(!allowed, "an ungranted capability must be denied");
}

#[tokio::test]
async fn one_grant_does_not_imply_another() {
    let handle = open_gate_store("check_distinct").await;
    let grantor = admin();
    let grantee = operator("alice");

    create_grant(handle.raw(), &grantor, &grantee, Capability::RuleInvoke)
        .await
        .expect("create grant");

    let other = check_capability(handle.raw(), &grantee, Capability::IngestPublish)
        .await
        .expect("check");
    assert!(!other, "holding one capability must not grant another");
}
