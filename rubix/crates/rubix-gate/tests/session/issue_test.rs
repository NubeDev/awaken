//! Integration: `issue_scoped_session` mints a session bound to the principal
//! by signing a connection clone in through the record access method.

#[path = "../gate/mod.rs"]
mod gate;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_gate::{PrincipalToken, authenticate, issue_scoped_session, provision_principal};

use gate::open::{NS, open_gate_store};

#[tokio::test]
async fn issued_session_is_bound_to_the_principal() {
    let database = "issue_bind";
    let handle = open_gate_store(database).await;
    let principal = Principal::new(Id::from_raw("alice"), NS, PrincipalKind::User, Role::Viewer);
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision");

    let token = PrincipalToken::new("alice", "pw");
    let resolved = authenticate(handle.raw(), &token)
        .await
        .expect("authenticate");
    let session = issue_scoped_session(handle.raw(), NS, database, resolved.clone(), &token)
        .await
        .expect("issue scoped session");

    assert_eq!(session.principal(), &resolved);
}

#[tokio::test]
async fn issuing_with_a_bad_secret_fails() {
    let database = "issue_bad_secret";
    let handle = open_gate_store(database).await;
    let principal = Principal::new(Id::from_raw("alice"), NS, PrincipalKind::User, Role::Viewer);
    provision_principal(handle.raw(), &principal, "pw")
        .await
        .expect("provision");

    let bad = PrincipalToken::new("alice", "nope");
    let err = issue_scoped_session(handle.raw(), NS, database, principal, &bad)
        .await
        .expect_err("bad secret must fail signin");
    assert!(err.to_string().contains("issue scoped session"));
}
