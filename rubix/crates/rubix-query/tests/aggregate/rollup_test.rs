//! Integration: a window rollup aggregates a scoped series per epoch bucket.
//!
//! Proves the vectorized time-window aggregation (`rubix/docs/sessions/WS-09.md`):
//! a numeric series read through the scoped session is rolled up into
//! epoch-aligned buckets, with `avg/min/max/sum/count/first/last` correct per
//! bucket — the values that feed a rule decision (`rubix/STACK-DEISGN.md`, "Rhai
//! owns the decision; DataFusion owns the data").

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_core::{Id, Record, Role, create_record};
use rubix_query::{CanonicalTable, Grain, rollup_window};
use surrealdb::types::Datetime;

use fixture::open::{open_query_store, scoped_session_for};

/// Seed a record in `namespace` with a chosen `created` instant and `temp`.
async fn seed(handle: &rubix_store::StoreHandle, at_secs: i64, temp: f64) {
    let mut record = Record::new("rubix", serde_json::json!({ "temp": temp }));
    record.id = Id::new();
    record.created = Datetime::from_timestamp(at_secs, 0).expect("valid instant");
    record.updated = record.created;
    create_record(handle.raw(), &record).await.expect("seed");
}

#[tokio::test]
async fn a_minute_rollup_computes_every_function_per_bucket() {
    let database = "rollup_minute";
    let handle = open_query_store(database).await;

    // Two minute buckets: [0,60) has 10,20,30; [60,120) has 5.
    seed(&handle, 0, 10.0).await;
    seed(&handle, 20, 20.0).await;
    seed(&handle, 40, 30.0).await;
    seed(&handle, 90, 5.0).await;

    let (_principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Viewer).await;

    let buckets = rollup_window(
        session.connection(),
        CanonicalTable::Records,
        "temp",
        Grain::Minute,
    )
    .await
    .expect("rollup");

    assert_eq!(buckets.len(), 2, "two minute buckets");

    let first = buckets[0];
    assert_eq!(first.bucket_start, 0);
    assert!((first.avg - 20.0).abs() < f64::EPSILON);
    assert_eq!(first.min, 10.0);
    assert_eq!(first.max, 30.0);
    assert_eq!(first.sum, 60.0);
    assert_eq!(first.count, 3);
    assert_eq!(first.first, 10.0);
    assert_eq!(first.last, 30.0);

    let second = buckets[1];
    assert_eq!(second.bucket_start, Grain::Minute.width_micros());
    assert_eq!(second.count, 1);
    assert_eq!(second.avg, 5.0);
}

#[tokio::test]
async fn an_hour_rollup_groups_minute_samples_into_one_bucket() {
    let database = "rollup_hour";
    let handle = open_query_store(database).await;

    // Three samples within the first hour collapse to one hour bucket.
    seed(&handle, 0, 1.0).await;
    seed(&handle, 600, 2.0).await;
    seed(&handle, 3000, 3.0).await;

    let (_principal, session) =
        scoped_session_for(&handle, database, "bob", "rubix", Role::Viewer).await;

    let buckets = rollup_window(
        session.connection(),
        CanonicalTable::Records,
        "temp",
        Grain::Hour,
    )
    .await
    .expect("rollup");

    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].bucket_start, 0);
    assert_eq!(buckets[0].count, 3);
    assert_eq!(buckets[0].sum, 6.0);
}

#[tokio::test]
async fn a_series_with_no_numeric_field_yields_no_buckets() {
    let database = "rollup_empty";
    let handle = open_query_store(database).await;
    let mut record = Record::new("rubix", serde_json::json!({ "label": "warm" }));
    record.created = Datetime::from_timestamp(0, 0).unwrap();
    create_record(handle.raw(), &record).await.expect("seed");

    let (_principal, session) =
        scoped_session_for(&handle, database, "carol", "rubix", Role::Viewer).await;

    let buckets = rollup_window(
        session.connection(),
        CanonicalTable::Records,
        "temp",
        Grain::Minute,
    )
    .await
    .expect("rollup");
    assert!(buckets.is_empty(), "no numeric samples means no buckets");
}
