//! Integration: attach a tag, then a single-tag query returns the record.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{
    Record, Tag, attach_tag, create_record, create_tag, find_records_by_tags,
};

use open::open_memory_db;

#[tokio::test]
async fn attach_then_find_by_single_tag_returns_the_record() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    let tag = Tag::new("temperature");
    create_record(&db, &record).await.expect("create record");
    create_tag(&db, &tag).await.expect("create tag");

    attach_tag(&db, &record.id, &tag.id).await.expect("attach");

    let found = find_records_by_tags(&db, std::slice::from_ref(&tag.id))
        .await
        .expect("find by tag");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, record.id);
}

#[tokio::test]
async fn attach_is_idempotent() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    let tag = Tag::new("temperature");
    create_record(&db, &record).await.expect("create record");
    create_tag(&db, &tag).await.expect("create tag");

    attach_tag(&db, &record.id, &tag.id).await.expect("attach 1");
    attach_tag(&db, &record.id, &tag.id).await.expect("attach 2");

    // A duplicate edge would still match the single tag, but it must not break
    // intersection counting: the record carries exactly one tag.
    let found = find_records_by_tags(&db, std::slice::from_ref(&tag.id))
        .await
        .expect("find by tag");
    assert_eq!(found.len(), 1);
}

#[tokio::test]
async fn find_by_empty_tag_set_matches_nothing() {
    let db = open_memory_db().await;
    let record = Record::new("rubix", serde_json::json!({ "v": 1 }));
    create_record(&db, &record).await.expect("create record");

    let found = find_records_by_tags(&db, &[]).await.expect("find empty");
    assert!(found.is_empty());
}
