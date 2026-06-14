//! Integration: deleting a record removes it and clears its tag edges.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{
    Record, Tag, attach_tag, create_record, create_tag, delete_record, find_records_by_tags,
    read_record,
};

use open::open_memory_db;

#[tokio::test]
async fn delete_removes_the_record() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    create_record(&db, &record).await.expect("create record");

    delete_record(&db, &record.id).await.expect("delete record");

    let result = read_record(&db, &record.id).await.expect("read deleted");
    assert!(result.is_none());
}

#[tokio::test]
async fn delete_clears_tag_edges_so_traversal_drops_the_record() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    let tag = Tag::new("temperature");
    create_record(&db, &record).await.expect("create record");
    create_tag(&db, &tag).await.expect("create tag");
    attach_tag(&db, &record.id, &tag.id).await.expect("attach");

    // Sanity: the tag finds the record before deletion.
    let before = find_records_by_tags(&db, std::slice::from_ref(&tag.id))
        .await
        .expect("find before");
    assert_eq!(before.len(), 1);

    delete_record(&db, &record.id).await.expect("delete record");

    let after = find_records_by_tags(&db, std::slice::from_ref(&tag.id))
        .await
        .expect("find after");
    assert!(after.is_empty(), "no dangling edge keeps a deleted record");
}
