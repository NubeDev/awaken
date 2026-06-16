//! Integration: registering a connector inserts it under its id, fail closed.
//!
//! Adding a datasource is adding a registry entry (`rubix/docs/SCOPE.md`,
//! "Datasources"), gated by the WS-04 capability. These cover the happy path
//! (granted register inserts the id), the duplicate guard, the reserved native id,
//! and the ungranted deny.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_datasource::{
    DatasourceError, NATIVE_SURREAL_ID, Registry, SurrealConnector, list, register,
};
use rubix_gate::Capability;

use fixture::open::{grant, open_datasource_store, scoped_session_for};
use rubix_core::Role;

#[tokio::test]
async fn a_granted_principal_registers_a_connector_under_its_id() {
    let database = "register_ok";
    let handle = open_datasource_store(database).await;
    let (principal, session) = scoped_session_for(&handle, database, "alice", Role::Operator).await;
    grant(&handle, &principal, Capability::DatasourceRegister).await;

    let mut registry = Registry::with_native_default();
    let connector = SurrealConnector::new("mirror", "Mirror", session.connection().clone());
    register(&mut registry, handle.raw(), &principal, connector)
        .await
        .expect("register");

    assert!(registry.contains("mirror"));
    assert!(registry.contains(NATIVE_SURREAL_ID));
    let ids: Vec<&str> = list(&registry).iter().map(|c| c.id()).collect();
    assert!(ids.contains(&"mirror"));
    assert!(ids.contains(&NATIVE_SURREAL_ID));
}

#[tokio::test]
async fn an_ungranted_principal_cannot_register() {
    let database = "register_denied";
    let handle = open_datasource_store(database).await;
    let (principal, session) =
        scoped_session_for(&handle, database, "mallory", Role::Operator).await;

    let mut registry = Registry::with_native_default();
    let connector = SurrealConnector::new("mirror", "Mirror", session.connection().clone());
    let err = register(&mut registry, handle.raw(), &principal, connector)
        .await
        .expect_err("ungranted register must be denied");
    assert!(matches!(err, DatasourceError::Denied), "got {err:?}");
    assert!(!registry.contains("mirror"));
}

#[tokio::test]
async fn a_duplicate_id_is_refused() {
    let database = "register_dup";
    let handle = open_datasource_store(database).await;
    let (principal, session) = scoped_session_for(&handle, database, "alice", Role::Operator).await;
    grant(&handle, &principal, Capability::DatasourceRegister).await;

    let mut registry = Registry::with_native_default();
    register(
        &mut registry,
        handle.raw(),
        &principal,
        SurrealConnector::new("mirror", "Mirror", session.connection().clone()),
    )
    .await
    .expect("first register");

    let err = register(
        &mut registry,
        handle.raw(),
        &principal,
        SurrealConnector::new("mirror", "Mirror Again", session.connection().clone()),
    )
    .await
    .expect_err("duplicate id must be refused");
    assert!(
        matches!(err, DatasourceError::Duplicate(ref id) if id == "mirror"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn the_reserved_native_id_cannot_be_shadowed() {
    let database = "register_native";
    let handle = open_datasource_store(database).await;
    let (principal, session) = scoped_session_for(&handle, database, "alice", Role::Operator).await;
    grant(&handle, &principal, Capability::DatasourceRegister).await;

    let mut registry = Registry::with_native_default();
    let err = register(
        &mut registry,
        handle.raw(),
        &principal,
        SurrealConnector::new(NATIVE_SURREAL_ID, "Impostor", session.connection().clone()),
    )
    .await
    .expect_err("the native id must not be shadowable");
    assert!(matches!(err, DatasourceError::Duplicate(_)), "got {err:?}");
}
