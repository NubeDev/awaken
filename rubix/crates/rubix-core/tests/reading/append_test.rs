//! Integration: append readings, re-append idempotently, read back windowed.
//!
//! Exercises the real SurrealQL the data-plane append emits against a live engine:
//! the deterministic `(series, at)` id makes a re-append a no-op (no duplicate
//! row, original receive time preserved), and the windowed read returns the
//! samples in `at` order. This is the store-side half of the trend-collapse fix —
//! measurement time is what the read path orders and buckets on.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{Reading, append_readings, list_readings, read_reading, read_readings_window};
use surrealdb::types::Datetime;

use open::open_memory_db;

fn at(secs: i64) -> Datetime {
    Datetime::from_timestamp(secs, 0).expect("valid instant")
}

fn reading(series: &str, secs: i64, value: f64) -> Reading {
    Reading::new("rubix", series, at(secs), value, serde_json::json!({}))
}

#[tokio::test]
async fn append_then_window_read_round_trips_in_at_order() {
    let db = open_memory_db().await;
    // Out-of-order on the wire; the read must come back ordered by `at`.
    let samples = vec![
        reading("reg-1", 2_000, 21.0),
        reading("reg-1", 1_000, 20.0),
        reading("reg-1", 3_000, 22.0),
    ];
    let written = append_readings(&db, &samples).await.expect("append");
    assert_eq!(written, 3, "all three samples land");

    let window = read_readings_window(&db, "reg-1", &at(0), &at(10_000))
        .await
        .expect("windowed read");
    let values: Vec<f64> = window.iter().map(|r| r.value).collect();
    assert_eq!(values, [20.0, 21.0, 22.0], "ordered by measurement instant");
    // `series` decodes to the bare register id for a direct `series == id` join.
    assert!(window.iter().all(|r| r.series == "reg-1"));
}

#[tokio::test]
async fn re_append_of_the_same_sample_is_an_idempotent_no_op() {
    let db = open_memory_db().await;
    let first = reading("reg-1", 1_000, 20.0);
    append_readings(&db, std::slice::from_ref(&first))
        .await
        .expect("first append");
    let created = read_reading(&db, &first.id)
        .await
        .expect("read back")
        .expect("present")
        .created;

    // A re-built reading for the same (series, at) derives the same id, so the
    // second append overwrites in place rather than inserting a duplicate.
    let again = reading("reg-1", 1_000, 20.0);
    assert_eq!(again.id, first.id, "same (series, at) → same id");
    append_readings(&db, std::slice::from_ref(&again))
        .await
        .expect("re-append");

    let all = list_readings(&db).await.expect("list");
    assert_eq!(all.len(), 1, "re-append is a no-op, not a duplicate row");
    // The original receive time survives the re-append (created is not updated).
    assert_eq!(all[0].created, created, "created preserved across re-append");
}

#[tokio::test]
async fn the_window_excludes_samples_outside_its_bounds() {
    let db = open_memory_db().await;
    let samples = vec![
        reading("reg-1", 1_000, 1.0),
        reading("reg-1", 5_000, 5.0),
        reading("reg-1", 9_000, 9.0),
    ];
    append_readings(&db, &samples).await.expect("append");

    let window = read_readings_window(&db, "reg-1", &at(2_000), &at(6_000))
        .await
        .expect("windowed read");
    let values: Vec<f64> = window.iter().map(|r| r.value).collect();
    assert_eq!(values, [5.0], "only the in-window sample is returned");
}
