//! Live-database tests for the sqlx Postgres backend.
//!
//! These exercise the SQL-touching path the fake backend cannot: real binding,
//! `statement_timeout`, `row_to_json` shaping, and `information_schema`
//! introspection. They are `#[ignore]` because this environment has no
//! Postgres; the cap/parameter/single-statement/named-query logic is covered by
//! the unit tests against a fake backend.
//!
//! To run them, start a throwaway Timescale/Postgres and point the env at it:
//!
//! ```sh
//! docker run --rm -e POSTGRES_PASSWORD=pw -p 5433:5432 timescale/timescaledb:latest-pg16
//! RUBIX_DS_TEST_HOST=localhost RUBIX_DS_TEST_PORT=5433 \
//!   RUBIX_DS_TEST_DB=postgres RUBIX_DS_TEST_USER=postgres \
//!   RUBIX_DS_TEST_PASSWORD=pw \
//!   cargo test -p rubix-datasource --test live_postgres -- --ignored
//! ```
//!
//! A `SELECT`-only role is the production guarantee; these tests use the
//! superuser for setup convenience and assert read behavior only.

use std::time::Duration;

use rubix_datasource::{
    Caps, DatasourceEntry, DatasourceError, DatasourceRegistry, Param, PostgresBackend,
    PostgresConn, SqlBackend,
};

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// Build a backend from `RUBIX_DS_TEST_*` env, or skip if unset.
async fn backend() -> Option<PostgresBackend> {
    let conn = PostgresConn {
        host: env("RUBIX_DS_TEST_HOST")?,
        port: env("RUBIX_DS_TEST_PORT")?.parse().ok()?,
        database: env("RUBIX_DS_TEST_DB")?,
        user: env("RUBIX_DS_TEST_USER")?,
        password: env("RUBIX_DS_TEST_PASSWORD")?,
    };
    PostgresBackend::connect("test", &conn, 2).await.ok()
}

#[tokio::test]
#[ignore = "needs a live Postgres; see module docs"]
async fn binds_parameters_and_shapes_rows() {
    let b = backend().await.expect("set RUBIX_DS_TEST_* env");
    let raw = b
        .run(
            "SELECT $1::int AS n, $2::text AS label",
            &vec![Param::Int(42), Param::Text("hello".into())],
            Some(Duration::from_secs(5)),
            Some(10),
        )
        .await
        .unwrap();
    assert_eq!(raw.columns.iter().map(|c| &c.name).collect::<Vec<_>>(), ["n", "label"]);
    assert_eq!(raw.rows[0][0], serde_json::json!(42));
    assert_eq!(raw.rows[0][1], serde_json::json!("hello"));
}

#[tokio::test]
#[ignore = "needs a live Postgres; see module docs"]
async fn parameter_is_data_not_sql() {
    let b = backend().await.expect("set RUBIX_DS_TEST_* env");
    // A SQL-shaped string bound as a param is returned verbatim, never executed.
    let raw = b
        .run(
            "SELECT $1::text AS x",
            &vec![Param::Text("'; DROP TABLE t; --".into())],
            Some(Duration::from_secs(5)),
            None,
        )
        .await
        .unwrap();
    assert_eq!(raw.rows[0][0], serde_json::json!("'; DROP TABLE t; --"));
}

#[tokio::test]
#[ignore = "needs a live Postgres; see module docs"]
async fn statement_timeout_kills_runaway() {
    let b = backend().await.expect("set RUBIX_DS_TEST_* env");
    let err = b
        .run("SELECT pg_sleep(10)", &vec![], Some(Duration::from_millis(200)), None)
        .await
        .unwrap_err();
    assert!(matches!(err, DatasourceError::Backend { .. }));
}

#[tokio::test]
#[ignore = "needs a live Postgres; see module docs"]
async fn registry_executes_and_caps_through_the_pool() {
    let host = match env("RUBIX_DS_TEST_HOST") {
        Some(h) => h,
        None => return,
    };
    let entry: DatasourceEntry = serde_json::from_value(serde_json::json!({
        "id": "live",
        "connection": {
            "host": host,
            "port": env("RUBIX_DS_TEST_PORT").unwrap().parse::<u16>().unwrap(),
            "database": env("RUBIX_DS_TEST_DB").unwrap(),
            "user": env("RUBIX_DS_TEST_USER").unwrap(),
            "password": env("RUBIX_DS_TEST_PASSWORD").unwrap()
        },
        "caps": {"max_rows": 3, "max_bytes": 1048576, "max_duration_ms": 5000},
        "named_queries": [
            {"name": "series", "sql": "SELECT g AS n FROM generate_series(1, $1::int) g", "param_count": 1}
        ]
    }))
    .unwrap();
    let mut reg = DatasourceRegistry::new();
    reg.register_all(vec![entry]).await.unwrap();

    // Named query over the pool, truncated at the row cap (lenient path).
    let rs = reg
        .executor("live")
        .unwrap()
        .invoke_named("series", &vec![Param::Int(10)])
        .await
        .unwrap();
    assert_eq!(rs.rows.len(), 3);
    assert!(rs.breached);

    // Strict path turns the same breach into an error.
    let rs2 = reg
        .executor("live")
        .unwrap()
        .invoke_named("series", &vec![Param::Int(10)])
        .await
        .unwrap();
    assert!(matches!(
        reg.executor("live").unwrap().strict(rs2),
        Err(DatasourceError::CapBreached { .. })
    ));

    let _ = Caps::unbounded(); // surface is public
}

#[tokio::test]
#[ignore = "needs a live Postgres; see module docs"]
async fn describe_introspects_information_schema() {
    let b = backend().await.expect("set RUBIX_DS_TEST_* env");
    let blob = rubix_datasource::describe(&b, None).await.unwrap();
    // A fresh database still has system-visible user schemas; assert it parses.
    assert!(blob.tables.iter().all(|t| !t.name.is_empty()));
}
