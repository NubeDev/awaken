//! Feature-gated end-to-end: a unified query spans SurrealDB and live Postgres.
//!
//! The full datasource path against a real backend (`rubix/docs/sessions/WS-10.md`):
//! connect the Postgres connector to a live database, register it under the WS-04
//! `datasource-register` capability, then run one read-only `span` query — gated
//! on `external-query` — that reads the external tables addressed as
//! `"<id>"."<table>"`. Compiled only with `--features postgres`; skips cleanly when
//! `RUBIX_TEST_PG` is unset so the suite stays green without a database.
//!
//! Expects the demo schema seeded by `docs/testing/scenarios/datasource-e2e.sh`
//! (a `sensor_readings` table of 72 rows and a `rubix_datasource_probe` table).

#![cfg(feature = "postgres")]

#[path = "../fixture/mod.rs"]
mod fixture;

use datafusion::arrow::array::{Array, Int64Array, StringArray};
use rubix_core::Role;
use rubix_datasource::{DatasourceError, PostgresConnector, Registry, register, span};
use rubix_gate::Capability;
use rubix_query::ContextCache;

use fixture::open::{grant, open_datasource_store, scoped_session_for};

/// The env var supplying a reachable Postgres URL. Unset = skip.
const PG_ENV: &str = "RUBIX_TEST_PG";
/// The datasource id the external warehouse is registered under.
const WAREHOUSE: &str = "warehouse";

/// Connect the warehouse connector exposing the seeded demo tables.
async fn warehouse(url: &str) -> PostgresConnector {
    PostgresConnector::connect(
        WAREHOUSE,
        "Demo Warehouse",
        url,
        vec![
            "sensor_readings".to_owned(),
            "rubix_datasource_probe".to_owned(),
        ],
    )
    .await
    .expect("connect to Postgres")
}

#[tokio::test]
async fn a_query_spans_surrealdb_and_live_postgres() {
    let Ok(url) = std::env::var(PG_ENV) else {
        eprintln!("{PG_ENV} unset; skipping the Postgres span e2e");
        return;
    };

    let database = "pg_span_ok";
    let handle = open_datasource_store(database).await;
    let (principal, session) = scoped_session_for(&handle, database, "alice", Role::Operator).await;
    grant(&handle, &principal, Capability::DatasourceRegister).await;
    grant(&handle, &principal, Capability::ExternalQuery).await;

    let mut registry = Registry::with_native_default();
    register(
        &mut registry,
        handle.raw(),
        &principal,
        warehouse(&url).await,
    )
    .await
    .expect("register the warehouse connector");

    // Count the seeded telemetry rows across the federation boundary. A column
    // aggregate (`count(measure)`) is used rather than `count(*)`: the latter
    // projects zero columns, which trips a DataFusion/table-providers schema
    // mismatch on the external scan.
    let cache = ContextCache::default();
    let batches = span(
        &registry,
        handle.raw(),
        &session,
        &cache,
        "SELECT count(measure) AS n FROM \"warehouse\".\"sensor_readings\"",
    )
    .await
    .expect("span query over live Postgres");
    let n = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("count column is Int64")
        .value(0);
    assert_eq!(n, 72, "the seeded sensor_readings table has 72 rows");

    // An aggregate pushed to the external source: group telemetry by measure.
    let grouped = span(
        &registry,
        handle.raw(),
        &session,
        &cache,
        "SELECT measure, count(measure) AS n FROM \"warehouse\".\"sensor_readings\" \
         GROUP BY measure ORDER BY measure",
    )
    .await
    .expect("grouped span query");
    let measures: Vec<String> = grouped
        .iter()
        .flat_map(|b| {
            let col = b
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("measure column is Utf8")
                .clone();
            (0..col.len()).map(move |i| col.value(i).to_owned())
        })
        .collect();
    assert!(
        measures.contains(&"temp".to_owned())
            && measures.contains(&"kw".to_owned())
            && measures.contains(&"flow".to_owned()),
        "expected the three seeded measures, got {measures:?}"
    );
}

#[tokio::test]
async fn the_span_query_fails_closed_without_external_query() {
    let Ok(url) = std::env::var(PG_ENV) else {
        eprintln!("{PG_ENV} unset; skipping the Postgres deny e2e");
        return;
    };

    let database = "pg_span_denied";
    let handle = open_datasource_store(database).await;
    let (principal, session) =
        scoped_session_for(&handle, database, "mallory", Role::Operator).await;
    // Granted register but NOT external-query: registration succeeds, the query denies.
    grant(&handle, &principal, Capability::DatasourceRegister).await;

    let mut registry = Registry::with_native_default();
    register(
        &mut registry,
        handle.raw(),
        &principal,
        warehouse(&url).await,
    )
    .await
    .expect("register the warehouse connector");

    let cache = ContextCache::default();
    let err = span(
        &registry,
        handle.raw(),
        &session,
        &cache,
        "SELECT count(*) FROM \"warehouse\".\"sensor_readings\"",
    )
    .await
    .expect_err("a query without external-query must be denied");
    assert!(matches!(err, DatasourceError::Denied), "got {err:?}");
}
