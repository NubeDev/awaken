//! Integration: the `trace_summary` rollup is queryable through the surface.
//!
//! §5b (`rubix/docs/design/LAMINAR-BORROW.md`) folds one `trace_summary` row per
//! correlation id; this proves that surface is now a canonical query table — a
//! `SELECT json_get(content, …) … GROUP BY` planned by DataFusion returns the
//! rolled-up fields, and only the principal's own tenant's rows (contract #1),
//! exactly as `record`/`audit` behave.

#[path = "../fixture/mod.rs"]
mod fixture;

use datafusion::arrow::array::{Array, Int64Array, StringArray};
use rubix_core::CorrelationId;
use rubix_query::run;
use rubix_trace::{Span, define_trace_schema, upsert_summary};
use rubix_trace::{MetricsBuilder, SpanStatus};

use fixture::open::{open_query_store, scoped_session_for};

/// A span carrying the §5a reserved metrics, ready to fold into a summary.
fn span(trace: &str, name: &str, status: SpanStatus, tokens: i64, dur: i64) -> Span {
    let mut attrs = serde_json::json!({});
    MetricsBuilder::new()
        .kind("rule")
        .status(status)
        .tokens(tokens)
        .cost(0.0)
        .apply(&mut attrs);
    Span::root(CorrelationId::carry(trace), name, attrs, 0, dur)
}

#[tokio::test]
async fn a_grouped_select_over_trace_summary_returns_rolled_up_rows() {
    let database = "query_trace_summary";
    let handle = open_query_store(database).await;
    // The rollup surface is defined by `rubix-trace`, not the gate/audit schema
    // the fixture applies — define it on the root handle before folding.
    define_trace_schema(handle.raw())
        .await
        .expect("define trace schema");

    // Two of the principal's traces (one ok, one errored) plus a second tenant's
    // trace that the scoped session must never see.
    upsert_summary(handle.raw(), "rubix", &span("t-ok", "plan", SpanStatus::Ok, 10, 5))
        .await
        .expect("seed ok trace");
    upsert_summary(handle.raw(), "rubix", &span("t-err", "plan", SpanStatus::Error, 20, 5))
        .await
        .expect("seed errored trace");
    upsert_summary(handle.raw(), "tenant-b", &span("t-other", "plan", SpanStatus::Ok, 99, 5))
        .await
        .expect("seed other tenant trace");

    let (_principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", rubix_core::Role::Viewer).await;

    // status counts: the json_get fields come back as text, count(*) aggregates.
    let batches = run(
        session.connection(),
        "SELECT json_get(content, 'status') AS status, count(*) AS n \
         FROM trace_summary GROUP BY status ORDER BY status",
    )
    .await
    .expect("run trace_summary aggregation");

    let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(rows, 2, "the principal's two trace statuses roll up; tenant-b is unseen");

    let batch = batches.first().expect("one batch");
    let statuses = batch
        .column_by_name("status")
        .expect("status column")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("utf8 status");
    let seen: Vec<&str> = (0..statuses.len()).map(|i| statuses.value(i)).collect();
    assert!(seen.contains(&"ok"), "the ok trace is rolled up: {seen:?}");
    assert!(seen.contains(&"error"), "the errored trace is rolled up: {seen:?}");
}

#[tokio::test]
async fn cast_aggregates_a_numeric_trace_field() {
    let database = "query_trace_tokens";
    let handle = open_query_store(database).await;
    define_trace_schema(handle.raw())
        .await
        .expect("define trace schema");

    // One trace, two folded spans → total_tokens = 10 + 25 = 35.
    upsert_summary(handle.raw(), "rubix", &span("t1", "a", SpanStatus::Ok, 10, 5))
        .await
        .expect("seed first span");
    upsert_summary(handle.raw(), "rubix", &span("t1", "b", SpanStatus::Ok, 25, 9))
        .await
        .expect("fold second span");

    let (_principal, session) =
        scoped_session_for(&handle, database, "bob", "rubix", rubix_core::Role::Viewer).await;

    // Numbers come back from json_get as text; CAST(... AS BIGINT) sums them.
    let batches = run(
        session.connection(),
        "SELECT CAST(json_get(content, 'total_tokens') AS BIGINT) AS tokens \
         FROM trace_summary",
    )
    .await
    .expect("run trace token cast");

    let batch = batches.first().expect("one batch");
    assert_eq!(batch.num_rows(), 1, "one trace, one rolled-up row");
    let tokens = batch
        .column_by_name("tokens")
        .expect("tokens column")
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("bigint tokens");
    assert_eq!(tokens.value(0), 35, "folded span tokens summed in the rollup");
}
