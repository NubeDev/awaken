//! Integration: a read-only SELECT over records returns the scoped rows.
//!
//! Proves the unification surface (`rubix/docs/sessions/WS-09.md`): a `SELECT`
//! planned by DataFusion over the canonical `record` table, scanned through the
//! principal's scoped session, returns rows — and only the rows SurrealDB
//! permissions admit, never another namespace's (contract #1).

#[path = "../fixture/mod.rs"]
mod fixture;

use datafusion::arrow::array::{Array, StringArray};
use rubix_core::{Record, Role, create_record};
use rubix_query::run;

use fixture::open::{open_query_store, scoped_session_for};

#[tokio::test]
async fn a_select_over_records_returns_scoped_rows() {
    let database = "query_run";
    let handle = open_query_store(database).await;

    // Two namespaces' data live in one database; the principal is scoped to A.
    for record in [
        Record::new("rubix", serde_json::json!({ "temp": 21 })),
        Record::new("rubix", serde_json::json!({ "temp": 22 })),
        Record::new("tenant-b", serde_json::json!({ "temp": 99 })),
    ] {
        create_record(handle.raw(), &record).await.expect("seed");
    }

    let (_principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Viewer).await;

    let batches = run(session.connection(), "SELECT id, namespace FROM record")
        .await
        .expect("run select");

    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(rows, 2, "scoped session sees only its namespace rows");

    for batch in &batches {
        let ns = batch
            .column_by_name("namespace")
            .expect("namespace column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("utf8 namespace");
        for i in 0..ns.len() {
            assert_eq!(ns.value(i), "rubix");
        }
    }
}

#[tokio::test]
async fn a_count_aggregation_runs_in_datafusion() {
    let database = "query_count";
    let handle = open_query_store(database).await;
    for record in [
        Record::new("rubix", serde_json::json!({ "k": 1 })),
        Record::new("rubix", serde_json::json!({ "k": 2 })),
        Record::new("rubix", serde_json::json!({ "k": 3 })),
    ] {
        create_record(handle.raw(), &record).await.expect("seed");
    }

    let (_principal, session) =
        scoped_session_for(&handle, database, "bob", "rubix", Role::Viewer).await;

    let batches = run(session.connection(), "SELECT count(*) AS n FROM record")
        .await
        .expect("run count");
    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(rows, 1, "a scalar aggregate returns one row");
}

#[tokio::test]
async fn json_get_reaches_into_the_content_document() {
    let database = "query_json_get";
    let handle = open_query_store(database).await;
    for record in [
        Record::new("rubix", serde_json::json!({ "kind": "reading", "v": 1 })),
        Record::new("rubix", serde_json::json!({ "kind": "reading", "v": 2 })),
        Record::new("rubix", serde_json::json!({ "kind": "board", "v": 3 })),
    ] {
        create_record(handle.raw(), &record).await.expect("seed");
    }

    let (_principal, session) =
        scoped_session_for(&handle, database, "dave", "rubix", Role::Viewer).await;

    // The `content` column is the whole row JSON, so the document payload sits
    // under `content` — a second json_get descends into it to reach `kind`.
    let batches = run(
        session.connection(),
        "SELECT json_get(json_get(content, 'content'), 'kind') AS kind, count(*) AS n \
         FROM record GROUP BY kind ORDER BY n DESC",
    )
    .await
    .expect("run json_get aggregation");

    let batch = batches.first().expect("one batch");
    let kinds = batch
        .column_by_name("kind")
        .expect("kind column")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("utf8 kind");
    let collected: Vec<&str> = (0..kinds.len()).map(|i| kinds.value(i)).collect();
    assert!(collected.contains(&"reading"), "json_get extracted the kind field");
    assert!(collected.contains(&"board"));
}

#[tokio::test]
async fn an_empty_canonical_table_returns_no_rows() {
    let database = "query_empty";
    let handle = open_query_store(database).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "carol", "rubix", Role::Viewer).await;

    // `insight` is declared but no writer has populated it yet.
    let batches = run(session.connection(), "SELECT * FROM insight")
        .await
        .expect("run over empty table");
    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(rows, 0);
}
