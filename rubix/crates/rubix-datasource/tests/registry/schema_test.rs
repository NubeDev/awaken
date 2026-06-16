//! Integration: `schema_of` lists the readable tables + columns (§4b).
//!
//! The shape-only twin of the span test (`rubix/docs/design/DASHBOARDS-SCOPE.md`
//! §4b): build the spanning context for a principal and enumerate the catalog —
//! the native canonical tables (with their structural columns) plus any registered
//! external datasource's tables, schema-qualified by id. Like `span`, it is gated
//! on `external-query` and fails closed.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_core::Role;
use rubix_datasource::{DatasourceError, Registry, SurrealConnector, register, schema_of};
use rubix_gate::Capability;
use rubix_query::ContextCache;

use fixture::open::{grant, open_datasource_store, scoped_session_for};

/// A granted principal sees the native canonical tables with their columns, and a
/// registered external datasource's tables appear under its id.
#[tokio::test]
async fn the_schema_lists_native_and_external_tables() {
    let database = "schema_listing";
    let handle = open_datasource_store(database).await;

    let (principal, session) = scoped_session_for(&handle, database, "alice", Role::Operator).await;
    grant(&handle, &principal, Capability::DatasourceRegister).await;
    grant(&handle, &principal, Capability::ExternalQuery).await;

    let mut registry = Registry::with_native_default();
    register(
        &mut registry,
        handle.raw(),
        &principal,
        SurrealConnector::new("mirror", "Mirror", session.connection().clone()),
    )
    .await
    .expect("register mirror");

    let cache = ContextCache::default();
    let tables = schema_of(&registry, handle.raw(), &session, &cache)
        .await
        .expect("schema enumerates");

    // The native canonical `record` table is present, bare (default schema), with
    // the structural columns the canonical schema declares.
    let record = tables
        .iter()
        .find(|t| t.table == "record" && t.schema != "mirror")
        .expect("native record table is listed");
    let columns: Vec<&str> = record.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        columns,
        ["id", "namespace", "created", "updated", "content"]
    );
    // Coarse type tags match the result-column vocabulary.
    let created = record.columns.iter().find(|c| c.name == "created").unwrap();
    assert_eq!(created.kind, "timestamp");

    // The external connector's tables are schema-qualified under its id.
    assert!(
        tables.iter().any(|t| t.schema == "mirror"),
        "the registered mirror datasource's tables are listed under `mirror`",
    );
}

/// Enumerating the schema requires the same `external-query` capability as `span`.
#[tokio::test]
async fn the_schema_is_gated_on_external_query() {
    let database = "schema_ungranted";
    let handle = open_datasource_store(database).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "mallory", Role::Operator).await;

    let registry = Registry::with_native_default();
    let cache = ContextCache::default();
    let err = schema_of(&registry, handle.raw(), &session, &cache)
        .await
        .expect_err("ungranted schema must be denied");
    assert!(matches!(err, DatasourceError::Denied), "got {err:?}");
}
