//! Integration: `list_record_tags` projects each record's tag names from the
//! `record→tagged→tag` graph against a live kv-mem SurrealDB.
//!
//! This is the read-only projection the wire join relies on (RecordDto.tags): a
//! record with no tags maps to an empty set, a tagged record maps to its tag
//! names, and the result keys by the record's string id for an O(1) join.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{Record, Tag, attach_tag, create_record, create_tag, list_record_tags};

use open::open_memory_db;

#[tokio::test]
async fn projects_tag_names_per_record() {
    let db = open_memory_db().await;

    let one = Tag::new("alpha");
    let two = Tag::new("beta");
    create_tag(&db, &one).await.expect("tag alpha");
    create_tag(&db, &two).await.expect("tag beta");

    let tagged = Record::new("rubix", serde_json::json!({ "kind": "thing", "n": 1 }));
    let untagged = Record::new("rubix", serde_json::json!({ "kind": "thing", "n": 2 }));
    create_record(&db, &tagged).await.expect("create tagged");
    create_record(&db, &untagged).await.expect("create untagged");

    attach_tag(&db, &tagged.id, &one.id).await.expect("attach alpha");
    attach_tag(&db, &tagged.id, &two.id).await.expect("attach beta");

    let by_id = list_record_tags(&db).await.expect("project tags");

    // The tagged record carries both names (order-insensitive).
    let mut tags = by_id
        .get(tagged.id.as_str())
        .cloned()
        .expect("tagged record present");
    tags.sort();
    assert_eq!(tags, vec!["alpha".to_owned(), "beta".to_owned()]);

    // The untagged record is present with an empty set, never absent.
    assert_eq!(
        by_id.get(untagged.id.as_str()).cloned().unwrap_or_default(),
        Vec::<String>::new()
    );
}
