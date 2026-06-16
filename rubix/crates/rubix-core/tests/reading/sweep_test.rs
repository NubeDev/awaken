//! Integration: the retention sweep is a per-namespace, `at`-bounded range delete.
//!
//! Exercises the real `DELETE … WHERE namespace = $ns AND at < $cutoff` against a
//! live engine: it drops only the targeted partition's pre-cutoff rows, leaves the
//! cutoff instant itself (exclusive bound) and every other namespace untouched, and
//! reports an exact deleted count.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{Reading, append_readings, list_readings, sweep_readings_before};
use surrealdb::types::Datetime;

use open::open_memory_db;

fn at(secs: i64) -> Datetime {
    Datetime::from_timestamp(secs, 0).expect("valid instant")
}

fn reading(namespace: &str, series: &str, secs: i64) -> Reading {
    Reading::new(namespace, series, at(secs), secs as f64, serde_json::json!({}))
}

#[tokio::test]
async fn sweep_drops_only_pre_cutoff_rows_in_the_named_partition() {
    let db = open_memory_db().await;
    append_readings(
        &db,
        &[
            reading("edge-a", "reg-1", 1_000),
            reading("edge-a", "reg-1", 2_000),
            reading("edge-a", "reg-1", 3_000),
            // A different partition must be left entirely alone.
            reading("edge-b", "reg-9", 1_000),
        ],
    )
    .await
    .expect("append");

    // Cutoff at 2_000 is exclusive: the 1_000 sample goes, 2_000 and 3_000 stay.
    let deleted = sweep_readings_before(&db, "edge-a", &at(2_000))
        .await
        .expect("sweep");
    assert_eq!(deleted, 1, "only the single pre-cutoff edge-a row is removed");

    let mut survivors: Vec<(String, f64)> = list_readings(&db)
        .await
        .expect("list")
        .into_iter()
        .map(|r| (r.namespace, r.value))
        .collect();
    survivors.sort_by(|a, b| a.partial_cmp(b).expect("total order"));
    assert_eq!(
        survivors,
        vec![
            ("edge-a".to_owned(), 2_000.0),
            ("edge-a".to_owned(), 3_000.0),
            ("edge-b".to_owned(), 1_000.0),
        ],
        "cutoff instant kept, other partition untouched",
    );
}

#[tokio::test]
async fn sweeping_a_partition_with_nothing_to_age_out_removes_nothing() {
    let db = open_memory_db().await;
    append_readings(&db, &[reading("edge-a", "reg-1", 5_000)])
        .await
        .expect("append");

    let deleted = sweep_readings_before(&db, "edge-a", &at(1_000))
        .await
        .expect("sweep");
    assert_eq!(deleted, 0, "nothing older than the cutoff");
    assert_eq!(list_readings(&db).await.expect("list").len(), 1, "row survives");
}
