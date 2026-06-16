//! Integration: `list_grants` returns a principal's grants, scoped to its
//! namespace.
//!
//! A grant only counts for the identity in its own tenant: a principal in
//! namespace A never sees a same-subject grant created in namespace B
//! (`rubix/docs/SCOPE.md`, "Two authz layers").

#[path = "../../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, create_grant, list_grants};

use gate::open::open_gate_store;

fn admin(namespace: &str) -> Principal {
    Principal::new(
        Id::from_raw("root"),
        namespace,
        PrincipalKind::User,
        Role::Admin,
    )
}

fn grantee(namespace: &str) -> Principal {
    Principal::new(
        Id::from_raw("alice"),
        namespace,
        PrincipalKind::User,
        Role::Operator,
    )
}

#[tokio::test]
async fn list_returns_every_held_grant() {
    let handle = open_gate_store("grant_list").await;
    let grantee = grantee("tenant-a");
    let grantor = admin("tenant-a");

    create_grant(handle.raw(), &grantor, &grantee, Capability::RuleInvoke)
        .await
        .expect("grant rule-invoke");
    create_grant(handle.raw(), &grantor, &grantee, Capability::ExternalQuery)
        .await
        .expect("grant external-query");

    let mut held: Vec<Capability> = list_grants(handle.raw(), &grantee)
        .await
        .expect("list")
        .into_iter()
        .map(|grant| grant.capability)
        .collect();
    held.sort_by_key(|capability| capability.as_str());
    assert_eq!(
        held,
        vec![Capability::ExternalQuery, Capability::RuleInvoke]
    );
}

#[tokio::test]
async fn list_is_scoped_to_the_principals_namespace() {
    let handle = open_gate_store("grant_list_scope").await;

    // Same subject, granted in tenant-b only.
    create_grant(
        handle.raw(),
        &admin("tenant-b"),
        &grantee("tenant-b"),
        Capability::RuleInvoke,
    )
    .await
    .expect("grant in tenant-b");

    // The same subject in tenant-a holds nothing.
    let in_a = list_grants(handle.raw(), &grantee("tenant-a"))
        .await
        .expect("list tenant-a");
    assert!(
        in_a.is_empty(),
        "a grant in another namespace must not leak"
    );

    let in_b = list_grants(handle.raw(), &grantee("tenant-b"))
        .await
        .expect("list tenant-b");
    assert_eq!(in_b.len(), 1);
}
