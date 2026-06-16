//! Integration: migrate legacy `kind:"history"` records into the `reading` plane.
//!
//! Exercises the documented one-shot migration against a live engine: well-formed
//! history records map (`ts → at`, `register → series`, `value → value`), append
//! into `reading`, and are deleted from `record`; malformed records are left in
//! place; and a second run is an idempotent no-op (deterministic `(series, at)`
//! ids, source records already gone).

#[path = "../db/open.rs"]
mod open;

use rubix_core::{
    HistoryMigration, Record, create_record, list_readings, list_records_filtered,
    migrate_history_to_readings,
};

use open::open_memory_db;

fn history(register: &str, ts: &str, value: f64) -> Record {
    Record::new(
        "rubix",
        serde_json::json!({
            "kind": "history",
            "register": register,
            "ts": ts,
            "value": value,
        }),
    )
}

#[tokio::test]
async fn well_formed_history_moves_into_the_reading_plane_and_leaves_record() {
    let db = open_memory_db().await;
    for record in [
        history("reg-1", "2026-06-14T10:00:00Z", 20.0),
        history("reg-1", "2026-06-14T11:00:00Z", 21.0),
        history("reg-2", "2026-06-14T10:00:00Z", 5.0),
    ] {
        create_record(&db, &record).await.expect("seed history");
    }

    let report = migrate_history_to_readings(&db).await.expect("migrate");
    assert_eq!(
        report,
        HistoryMigration { migrated: 3, deleted: 3, skipped: 0 }
    );

    // Readings landed, mapped to the bare register id, ordered by `at`.
    let readings = list_readings(&db).await.expect("list readings");
    let mapped: Vec<(String, f64)> = readings
        .iter()
        .map(|r| (r.series.clone(), r.value))
        .collect();
    assert_eq!(
        mapped,
        vec![
            ("reg-1".to_owned(), 20.0),
            ("reg-2".to_owned(), 5.0),
            ("reg-1".to_owned(), 21.0),
        ],
    );
    // The source records are gone — the `record` table holds no more history.
    let left = list_records_filtered(&db, Some("history"), &[])
        .await
        .expect("list history");
    assert!(left.is_empty(), "migrated history records deleted");
}

#[tokio::test]
async fn malformed_history_is_skipped_and_left_in_place() {
    let db = open_memory_db().await;
    create_record(&db, &history("reg-1", "2026-06-14T10:00:00Z", 20.0))
        .await
        .expect("good");
    // Missing `register`, bad timestamp, and non-numeric value — each unmigratable.
    create_record(
        &db,
        &Record::new("rubix", serde_json::json!({ "kind": "history", "ts": "2026-06-14T10:00:00Z", "value": 1.0 })),
    )
    .await
    .expect("no register");
    create_record(
        &db,
        &Record::new("rubix", serde_json::json!({ "kind": "history", "register": "reg-9", "ts": "not-a-time", "value": 1.0 })),
    )
    .await
    .expect("bad ts");

    let report = migrate_history_to_readings(&db).await.expect("migrate");
    assert_eq!(report.migrated, 1, "only the well-formed record migrates");
    assert_eq!(report.skipped, 2, "the two malformed records are skipped");

    let left = list_records_filtered(&db, Some("history"), &[])
        .await
        .expect("list history");
    assert_eq!(left.len(), 2, "skipped records stay in the record table");
}

#[tokio::test]
async fn re_running_the_migration_is_an_idempotent_no_op() {
    let db = open_memory_db().await;
    create_record(&db, &history("reg-1", "2026-06-14T10:00:00Z", 20.0))
        .await
        .expect("seed");

    let first = migrate_history_to_readings(&db).await.expect("first run");
    assert_eq!(first.migrated, 1);

    // Nothing left to migrate; the reading is not duplicated.
    let second = migrate_history_to_readings(&db).await.expect("second run");
    assert_eq!(
        second,
        HistoryMigration { migrated: 0, deleted: 0, skipped: 0 }
    );
    assert_eq!(list_readings(&db).await.expect("list").len(), 1, "no duplicate reading");
}
