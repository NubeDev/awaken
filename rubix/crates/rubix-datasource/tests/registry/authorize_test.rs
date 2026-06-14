//! Integration: registering a datasource is gated by `datasource-register`.
//!
//! Registration is an app-enforced capability (`rubix/STACK-DEISGN.md`, contract
//! #2): a principal holding the `datasource-register` grant is authorized; the
//! same principal without it is denied. Fail closed — the default is deny.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_datasource::{DatasourceError, authorize_register};
use rubix_gate::Capability;

use fixture::open::{grant, open_datasource_store, scoped_session_for};
use rubix_core::Role;

#[tokio::test]
async fn a_granted_principal_is_authorized_to_register() {
    let database = "authz_register_granted";
    let handle = open_datasource_store(database).await;
    let (principal, _session) =
        scoped_session_for(&handle, database, "alice", Role::Operator).await;
    grant(&handle, &principal, Capability::DatasourceRegister).await;

    authorize_register(handle.raw(), &principal)
        .await
        .expect("granted principal authorized");
}

#[tokio::test]
async fn an_ungranted_principal_is_denied() {
    let database = "authz_register_denied";
    let handle = open_datasource_store(database).await;
    let (principal, _session) =
        scoped_session_for(&handle, database, "mallory", Role::Operator).await;

    let err = authorize_register(handle.raw(), &principal)
        .await
        .expect_err("ungranted register must be denied");
    assert!(matches!(err, DatasourceError::Denied), "got {err:?}");
}
