//! Integration: a principal cannot escalate privilege through grants.
//!
//! `rubix/docs/SCOPE.md` ("Capabilities are grants"): a principal cannot confer
//! a capability it lacks the authority to confer. The gate enforces this fail
//! closed — a non-admin grantor is refused, and even an admin cannot administer
//! grants outside its own namespace, so no grant can cross a tenant boundary or
//! be self-minted by a principal without authority.

#[path = "../../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, GateError, check_capability, create_grant, revoke_grant};

use gate::open::{NS, open_gate_store};

fn principal(subject: &str, namespace: &str, role: Role) -> Principal {
    Principal::new(Id::from_raw(subject), namespace, PrincipalKind::User, role)
}

#[tokio::test]
async fn a_non_admin_cannot_grant_itself_a_capability() {
    let handle = open_gate_store("escalate_self").await;
    let operator = principal("alice", NS, Role::Operator);

    let err = create_grant(
        handle.raw(),
        &operator,
        &operator,
        Capability::DatasourceRegister,
    )
    .await
    .expect_err("a non-admin must not self-grant");
    assert!(matches!(err, GateError::GrantDenied(_)));

    // No grant was written, so the check still denies.
    let allowed = check_capability(handle.raw(), &operator, Capability::DatasourceRegister)
        .await
        .expect("check");
    assert!(!allowed, "the refused grant must not have landed");
}

#[tokio::test]
async fn a_viewer_cannot_grant_another_principal() {
    let handle = open_gate_store("escalate_other").await;
    let viewer = principal("vic", NS, Role::Viewer);
    let target = principal("alice", NS, Role::Operator);

    let err = create_grant(handle.raw(), &viewer, &target, Capability::RuleInvoke)
        .await
        .expect_err("a viewer must not confer grants");
    assert!(matches!(err, GateError::GrantDenied(_)));
}

#[tokio::test]
async fn an_admin_cannot_grant_across_namespaces() {
    let handle = open_gate_store("escalate_cross_ns").await;
    let admin_a = principal("root", "tenant-a", Role::Admin);
    let foreign = principal("eve", "tenant-b", Role::Operator);

    let err = create_grant(handle.raw(), &admin_a, &foreign, Capability::IngestPublish)
        .await
        .expect_err("an admin must not grant outside its namespace");
    assert!(matches!(err, GateError::GrantDenied(_)));

    let allowed = check_capability(handle.raw(), &foreign, Capability::IngestPublish)
        .await
        .expect("check");
    assert!(!allowed, "no cross-namespace grant may land");
}

#[tokio::test]
async fn a_non_admin_cannot_revoke() {
    let handle = open_gate_store("escalate_revoke").await;
    let admin = principal("root", NS, Role::Admin);
    let operator = principal("alice", NS, Role::Operator);

    create_grant(handle.raw(), &admin, &operator, Capability::RuleInvoke)
        .await
        .expect("admin grants");

    let err = revoke_grant(handle.raw(), &operator, &operator, Capability::RuleInvoke)
        .await
        .expect_err("a non-admin must not revoke");
    assert!(matches!(err, GateError::GrantDenied(_)));

    // The grant survives the refused revoke.
    let allowed = check_capability(handle.raw(), &operator, Capability::RuleInvoke)
        .await
        .expect("check");
    assert!(allowed, "the grant must remain after a refused revoke");
}
