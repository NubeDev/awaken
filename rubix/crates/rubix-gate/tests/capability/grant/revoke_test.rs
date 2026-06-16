//! Integration: `revoke_grant` removes a grant and a later check denies.
//!
//! Revoke is idempotent — revoking an absent grant succeeds — and a revoked
//! grant no longer satisfies `check_capability`.

#[path = "../../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, check_capability, create_grant, list_grants, revoke_grant};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn grantee() -> Principal {
    Principal::new(
        Id::from_raw("alice"),
        NS,
        PrincipalKind::User,
        Role::Operator,
    )
}

#[tokio::test]
async fn revoke_removes_the_grant_and_denies_after() {
    let handle = open_gate_store("grant_revoke").await;
    create_grant(handle.raw(), &admin(), &grantee(), Capability::RuleInvoke)
        .await
        .expect("create");
    assert!(
        check_capability(handle.raw(), &grantee(), Capability::RuleInvoke)
            .await
            .expect("check before"),
        "the grant should be allowed before revoke",
    );

    revoke_grant(handle.raw(), &admin(), &grantee(), Capability::RuleInvoke)
        .await
        .expect("revoke");

    assert!(
        !check_capability(handle.raw(), &grantee(), Capability::RuleInvoke)
            .await
            .expect("check after"),
        "a revoked grant must be denied",
    );
    assert!(
        list_grants(handle.raw(), &grantee())
            .await
            .expect("list")
            .is_empty()
    );
}

#[tokio::test]
async fn revoking_an_absent_grant_is_a_noop() {
    let handle = open_gate_store("grant_revoke_absent").await;
    revoke_grant(handle.raw(), &admin(), &grantee(), Capability::RuleInvoke)
        .await
        .expect("revoke of an absent grant must succeed");
}
