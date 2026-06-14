//! Integration: create a record then read it back identically.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{Record, create_record, read_record};

use open::open_memory_db;

#[tokio::test]
async fn create_then_read_round_trips_the_record() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "temp": 21.5, "unit": "c" }));

    let created = create_record(&db, &record).await.expect("create record");
    assert_eq!(created, record);

    let fetched = read_record(&db, &record.id)
        .await
        .expect("read record")
        .expect("record present");
    assert_eq!(fetched, record);
}

#[tokio::test]
async fn read_absent_record_is_none() {
    let db = open_memory_db().await;
    let missing = rubix_core::Id::new();
    let result = read_record(&db, &missing).await.expect("read absent");
    assert!(result.is_none());
}
