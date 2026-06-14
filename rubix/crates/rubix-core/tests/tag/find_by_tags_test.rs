//! Integration: multi-tag intersection returns only full-set matches.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{Record, Tag, attach_tag, create_record, create_tag, find_records_by_tags};

use open::open_memory_db;

#[tokio::test]
async fn intersection_returns_only_records_with_the_full_tag_set() {
    let db = open_memory_db().await;

    let temperature = Tag::new("temperature");
    let floor_2 = Tag::new("floor-2");
    create_tag(&db, &temperature).await.expect("create temperature");
    create_tag(&db, &floor_2).await.expect("create floor-2");

    // Carries both tags — must match.
    let both = Record::new("rubix", serde_json::json!({ "name": "both" }));
    create_record(&db, &both).await.expect("create both");
    attach_tag(&db, &both.id, &temperature.id).await.expect("attach temp");
    attach_tag(&db, &both.id, &floor_2.id).await.expect("attach floor");

    // Carries only one tag — must be excluded from the intersection.
    let partial = Record::new("rubix", serde_json::json!({ "name": "partial" }));
    create_record(&db, &partial).await.expect("create partial");
    attach_tag(&db, &partial.id, &temperature.id).await.expect("attach temp");

    // Carries neither — must be excluded.
    let neither = Record::new("rubix", serde_json::json!({ "name": "neither" }));
    create_record(&db, &neither).await.expect("create neither");

    let found = find_records_by_tags(&db, &[temperature.id.clone(), floor_2.id.clone()])
        .await
        .expect("find by tag set");

    assert_eq!(found.len(), 1, "only the full-set record matches");
    assert_eq!(found[0].id, both.id);
}

#[tokio::test]
async fn single_tag_in_set_returns_all_carriers() {
    let db = open_memory_db().await;
    let tag = Tag::new("sensor");
    create_tag(&db, &tag).await.expect("create tag");

    let a = Record::new("rubix", serde_json::json!({ "n": "a" }));
    let b = Record::new("rubix", serde_json::json!({ "n": "b" }));
    create_record(&db, &a).await.expect("create a");
    create_record(&db, &b).await.expect("create b");
    attach_tag(&db, &a.id, &tag.id).await.expect("attach a");
    attach_tag(&db, &b.id, &tag.id).await.expect("attach b");

    let found = find_records_by_tags(&db, std::slice::from_ref(&tag.id))
        .await
        .expect("find by tag");
    assert_eq!(found.len(), 2);
}
