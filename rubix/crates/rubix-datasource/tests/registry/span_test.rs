//! Integration: a unified query spans SurrealDB and a registered connector.
//!
//! The load-bearing scope-test (`rubix/docs/sessions/WS-10.md`): register a second
//! connector, run one read-only query that reads both the native SurrealDB tables
//! and the connector's tables, and assert the combined rows. Also: the query
//! action is gated on `external-query` (fail closed), and a query naming an
//! unregistered datasource fails to plan rather than escaping scope.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_core::{Record, Role, create_record};
use rubix_datasource::{DatasourceError, Registry, SurrealConnector, register, span};
use rubix_gate::Capability;

use fixture::open::{NS, grant, open_datasource_store, scoped_session_for};

/// Register a SurrealDB-backed second datasource and span it with the native one.
#[tokio::test]
async fn a_query_spans_surrealdb_and_the_registered_connector() {
    let database = "span_combined";
    let handle = open_datasource_store(database).await;
    for temp in [1, 2, 3] {
        create_record(handle.raw(), &Record::new(NS, serde_json::json!({ "temp": temp })))
            .await
            .expect("seed record");
    }

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

    // The native `record` table and the connector's `"mirror"."record"` table
    // both reflect the principal's scope; the union spans both sources.
    let sql = r#"SELECT id FROM record UNION ALL SELECT id FROM "mirror"."record""#;
    let batches = span(&registry, handle.raw(), &session, sql)
        .await
        .expect("spanning query runs");
    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(rows, 6, "three native rows + three mirrored rows");
}

#[tokio::test]
async fn the_spanning_query_is_gated_on_external_query() {
    let database = "span_ungranted";
    let handle = open_datasource_store(database).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "mallory", Role::Operator).await;

    let registry = Registry::with_native_default();
    let err = span(&registry, handle.raw(), &session, "SELECT id FROM record")
        .await
        .expect_err("ungranted span must be denied");
    assert!(matches!(err, DatasourceError::Denied), "got {err:?}");
}

#[tokio::test]
async fn a_query_naming_an_unregistered_datasource_fails_to_plan() {
    let database = "span_unknown";
    let handle = open_datasource_store(database).await;
    let (principal, session) = scoped_session_for(&handle, database, "alice", Role::Operator).await;
    grant(&handle, &principal, Capability::ExternalQuery).await;

    let registry = Registry::with_native_default();
    // `ghost` is not a registered datasource, so its table is not in the catalog;
    // DataFusion cannot resolve the name and the query fails rather than escaping.
    let err = span(
        &registry,
        handle.raw(),
        &session,
        r#"SELECT id FROM "ghost"."record""#,
    )
    .await
    .expect_err("an unregistered datasource must not resolve");
    assert!(matches!(err, DatasourceError::DataFusion(_)), "got {err:?}");
}
