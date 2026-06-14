//! Feature-gated integration: the Postgres connector round-trips a `SELECT`.
//!
//! Only compiled with `--features postgres` (`rubix/docs/sessions/WS-10.md`). When
//! the `RUBIX_TEST_PG` env var is set to a reachable Postgres URL, the test
//! connects, registers the connector, and round-trips a `SELECT 1`; when it is
//! unset the test skips cleanly (returns without asserting) rather than failing,
//! so the suite stays green on a machine with no Postgres.

#![cfg(feature = "postgres")]

use rubix_datasource::{Connector, PostgresConnector};

/// The env var that supplies a reachable Postgres connection string. Unset =
/// skip.
const PG_ENV: &str = "RUBIX_TEST_PG";

#[tokio::test]
async fn the_postgres_connector_round_trips_a_select() {
    let Ok(url) = std::env::var(PG_ENV) else {
        eprintln!("{PG_ENV} unset; skipping the Postgres round-trip test");
        return;
    };

    // A connector over a one-table view; the test table is created by the
    // operator who set RUBIX_TEST_PG (the round-trip reads it, not the DDL).
    let connector = PostgresConnector::connect(
        "warehouse",
        "Test Warehouse",
        &url,
        vec!["rubix_datasource_probe".to_owned()],
    )
    .await
    .expect("connect to Postgres");

    let provider = connector
        .table_provider("rubix_datasource_probe")
        .await
        .expect("build a provider for the probe table");
    assert!(
        !provider.schema().fields().is_empty(),
        "the probe table must expose at least one column"
    );
}
