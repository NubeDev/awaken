//! Integration: detaching a tag drops the record from that tag's results.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{
    Record, Tag, attach_tag, create_record, create_tag, detach_tag, find_records_by_tags,
};

use open::open_memory_db;

#[tokio::test]
async fn detach_removes_the_record_from_tag_results() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    let tag = Tag::new("temperature");
    create_record(&db, &record).await.expect("create record");
    create_tag(&db, &tag).await.expect("create tag");
    attach_tag(&db, &record.id, &tag.id).await.expect("attach");

    detach_tag(&db, &record.id, &tag.id).await.expect("detach");

    let found = find_records_by_tags(&db, std::slice::from_ref(&tag.id))
        .await
        .expect("find after detach");
    assert!(found.is_empty());
}

#[tokio::test]
async fn detach_absent_edge_is_a_noop() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    let tag = Tag::new("temperature");
    create_record(&db, &record).await.expect("create record");
    create_tag(&db, &tag).await.expect("create tag");

    // No edge was ever attached; detach must not error.
    detach_tag(&db, &record.id, &tag.id)
        .await
        .expect("detach noop");
}
