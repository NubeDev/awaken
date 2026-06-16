//! Integration: resolving a datasource id, fail closed on unknown.
//!
//! `resolve` is the lookup the spanning query uses. A registered external id
//! yields its materialised providers; the native id yields none (scanned per
//! query through the scoped session); an unknown id is denied with
//! [`DatasourceError::Unknown`] rather than silently treated as empty
//! (`rubix/docs/SCOPE.md`, "Datasources").

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_datasource::{
    DatasourceError, NATIVE_SURREAL_ID, Registry, SurrealConnector, register, resolve,
};
use rubix_gate::Capability;

use fixture::open::{grant, open_datasource_store, scoped_session_for};
use rubix_core::Role;

#[tokio::test]
async fn an_external_id_resolves_to_its_providers() {
    let database = "resolve_external";
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
    .expect("register");

    let providers = resolve(&registry, "mirror").expect("resolve mirror");
    // The SurrealDB connector offers every canonical table
    // (`CanonicalTable::ALL`): record, tag, audit, insight, trace_summary, reading.
    assert_eq!(providers.len(), 6);
}

#[tokio::test]
async fn the_native_id_resolves_to_no_stored_providers() {
    let registry = Registry::with_native_default();
    let providers = resolve(&registry, NATIVE_SURREAL_ID).expect("resolve native");
    assert!(providers.is_empty());
}

#[tokio::test]
async fn an_unknown_id_is_denied() {
    let registry = Registry::with_native_default();
    let err = resolve(&registry, "nope").expect_err("unknown id must be denied");
    assert!(
        matches!(err, DatasourceError::Unknown(ref id) if id == "nope"),
        "got {err:?}"
    );
}
