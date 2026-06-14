//! Integration: the query action is gated by the external-query capability.
//!
//! Querying the unified surface is app-enforced (`rubix/STACK-DEISGN.md`,
//! contract #2): a principal that holds the `external-query` grant may run a
//! query; the same principal without the grant is denied before any scan. Fail
//! closed — the default is deny.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_core::{Id, Principal, PrincipalKind, Record, Role, create_record};
use rubix_query::{QueryError, run_authorized};
use rubix_gate::{Capability, create_grant};

use fixture::open::{open_query_store, scoped_session_for};

fn admin() -> Principal {
    Principal::new(Id::from_raw("root"), "rubix", PrincipalKind::User, Role::Admin)
}

#[tokio::test]
async fn a_granted_principal_may_query() {
    let database = "authz_granted";
    let handle = open_query_store(database).await;
    create_record(
        handle.raw(),
        &Record::new("rubix", serde_json::json!({ "temp": 1 })),
    )
    .await
    .expect("seed");

    let (principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Operator).await;
    create_grant(handle.raw(), &admin(), &principal, Capability::ExternalQuery)
        .await
        .expect("grant external-query");

    let batches = run_authorized(handle.raw(), &session, "SELECT id FROM record")
        .await
        .expect("granted query runs");
    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn an_ungranted_principal_is_denied() {
    let database = "authz_denied";
    let handle = open_query_store(database).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "mallory", "rubix", Role::Operator).await;

    let err = run_authorized(handle.raw(), &session, "SELECT id FROM record")
        .await
        .expect_err("ungranted query must be denied");
    assert!(matches!(err, QueryError::Denied), "got {err:?}");
}

#[tokio::test]
async fn a_revoked_grant_no_longer_authorizes() {
    let database = "authz_revoked";
    let handle = open_query_store(database).await;
    let (principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Operator).await;

    create_grant(handle.raw(), &admin(), &principal, Capability::ExternalQuery)
        .await
        .expect("grant");
    rubix_gate::revoke_grant(handle.raw(), &admin(), &principal, Capability::ExternalQuery)
        .await
        .expect("revoke");

    let err = run_authorized(handle.raw(), &session, "SELECT id FROM record")
        .await
        .expect_err("a revoked grant must deny");
    assert!(matches!(err, QueryError::Denied), "got {err:?}");
}
