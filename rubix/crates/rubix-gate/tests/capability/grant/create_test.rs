//! Integration: `create_grant` persists a grant an admin confers.
//!
//! Creating a grant is idempotent (a deterministic key per principal +
//! capability), so conferring the same grant twice leaves one grant that a
//! subsequent check still allows.

#[path = "../../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{Capability, check_capability, create_grant, list_grants};

use gate::open::{NS, open_gate_store};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), NS, PrincipalKind::User, Role::Admin)
}

fn grantee() -> Principal {
    Principal::new(
        Id::from_raw("ext-1"),
        NS,
        PrincipalKind::Extension,
        Role::Operator,
    )
}

#[tokio::test]
async fn create_then_check_allows() {
    let handle = open_gate_store("grant_create").await;
    let grant = create_grant(
        handle.raw(),
        &admin(),
        &grantee(),
        Capability::IngestPublish,
    )
    .await
    .expect("create grant");

    assert_eq!(grant.subject, "ext-1");
    assert_eq!(grant.namespace, NS);
    assert_eq!(grant.capability, Capability::IngestPublish);

    let allowed = check_capability(handle.raw(), &grantee(), Capability::IngestPublish)
        .await
        .expect("check");
    assert!(allowed);
}

#[tokio::test]
async fn create_is_idempotent() {
    let handle = open_gate_store("grant_create_idempotent").await;
    create_grant(
        handle.raw(),
        &admin(),
        &grantee(),
        Capability::IngestPublish,
    )
    .await
    .expect("first create");
    create_grant(
        handle.raw(),
        &admin(),
        &grantee(),
        Capability::IngestPublish,
    )
    .await
    .expect("second create");

    let grants = list_grants(handle.raw(), &grantee()).await.expect("list");
    assert_eq!(grants.len(), 1, "the same grant must not duplicate");
}
