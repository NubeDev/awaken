//! Integration: `authenticate` maps a valid token to the right principal and
//! rejects an invalid one, for both user and extension kinds.

mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{PrincipalToken, authenticate, provision_principal};

use gate::open::open_gate_store;

#[tokio::test]
async fn valid_user_token_resolves_to_its_principal() {
    let handle = open_gate_store("auth_user").await;
    let principal = Principal::new(Id::from_raw("alice"), "tenant-a", PrincipalKind::User, Role::Viewer);
    provision_principal(handle.raw(), &principal, "s3cret")
        .await
        .expect("provision");

    let token = PrincipalToken::new("alice", "s3cret");
    let resolved = authenticate(handle.raw(), &token).await.expect("authenticate");

    assert_eq!(resolved.subject, principal.subject);
    assert_eq!(resolved.namespace, "tenant-a");
    assert_eq!(resolved.kind, PrincipalKind::User);
    assert_eq!(resolved.role, Role::Viewer);
}

#[tokio::test]
async fn valid_extension_token_resolves_to_an_extension_principal() {
    let handle = open_gate_store("auth_ext").await;
    let principal = Principal::new(
        Id::from_raw("ingestor"),
        "tenant-b",
        PrincipalKind::Extension,
        Role::Operator,
    );
    provision_principal(handle.raw(), &principal, "ext-secret")
        .await
        .expect("provision");

    let token = PrincipalToken::new("ingestor", "ext-secret");
    let resolved = authenticate(handle.raw(), &token).await.expect("authenticate");

    assert_eq!(resolved.kind, PrincipalKind::Extension);
    assert_eq!(resolved.namespace, "tenant-b");
    assert_eq!(resolved.role, Role::Operator);
}

#[tokio::test]
async fn wrong_secret_is_rejected() {
    let handle = open_gate_store("auth_wrong_secret").await;
    let principal = Principal::new(Id::from_raw("bob"), "tenant-a", PrincipalKind::User, Role::Admin);
    provision_principal(handle.raw(), &principal, "right")
        .await
        .expect("provision");

    let err = authenticate(handle.raw(), &PrincipalToken::new("bob", "wrong"))
        .await
        .expect_err("wrong secret must be rejected");
    assert!(err.to_string().contains("authentication failed"));
}

#[tokio::test]
async fn unknown_subject_is_rejected() {
    let handle = open_gate_store("auth_unknown").await;
    let err = authenticate(handle.raw(), &PrincipalToken::new("ghost", "whatever"))
        .await
        .expect_err("unknown subject must be rejected");
    assert!(err.to_string().contains("authentication failed"));
}
