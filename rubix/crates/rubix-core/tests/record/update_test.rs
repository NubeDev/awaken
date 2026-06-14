//! Integration: updating content bumps `updated` but preserves `created`.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{Record, create_record, read_record, update_record};

use open::open_memory_db;

#[tokio::test]
async fn update_replaces_content_and_bumps_updated() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    create_record(&db, &record).await.expect("create record");

    let updated = update_record(&db, &record.id, serde_json::json!({ "v": 2 }))
        .await
        .expect("update record")
        .expect("record present");

    assert_eq!(updated.content, serde_json::json!({ "v": 2 }));
    assert_eq!(updated.created, record.created, "created preserved");
    assert!(updated.updated >= record.updated, "updated bumped forward");

    let fetched = read_record(&db, &record.id)
        .await
        .expect("read record")
        .expect("record present");
    assert_eq!(fetched.content, serde_json::json!({ "v": 2 }));
}

#[tokio::test]
async fn update_absent_record_is_none() {
    let db = open_memory_db().await;
    let missing = rubix_core::Id::new();
    let result = update_record(&db, &missing, serde_json::json!({}))
        .await
        .expect("update absent");
    assert!(result.is_none());
}
