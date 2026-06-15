//! Integration: `list_records_filtered` narrows by collection kind and tag set
//! against a live kv-mem SurrealDB.
//!
//! Exercises the read-side narrowing the collection grids need: a `kind` filter
//! returns only that collection's records, a `tag` filter returns only records
//! carrying the whole requested tag set (Haystack intersection), and the two
//! compose. An absent filter is exactly `list_records`.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{
    Record, Tag, attach_tag, create_record, create_tag, list_records, list_records_filtered,
};

use open::open_memory_db;

#[tokio::test]
async fn kind_filter_returns_only_that_collection() {
    let db = open_memory_db().await;
    for content in [
        serde_json::json!({ "kind": "site", "name": "HQ" }),
        serde_json::json!({ "kind": "site", "name": "Depot" }),
        serde_json::json!({ "kind": "task", "title": "patrol" }),
    ] {
        create_record(&db, &Record::new("rubix", content))
            .await
            .expect("create");
    }

    let sites = list_records_filtered(&db, Some("site"), &[])
        .await
        .expect("filter sites");
    assert_eq!(sites.len(), 2);
    assert!(sites
        .iter()
        .all(|r| r.content.get("kind").and_then(|v| v.as_str()) == Some("site")));

    // No filter still lists everything.
    assert_eq!(list_records(&db).await.expect("all").len(), 3);
}

#[tokio::test]
async fn tag_filter_requires_the_whole_set() {
    let db = open_memory_db().await;

    let hvac = Tag::new("hvac");
    let floor2 = Tag::new("floor-2");
    create_tag(&db, &hvac).await.expect("tag hvac");
    create_tag(&db, &floor2).await.expect("tag floor-2");

    let both = Record::new("rubix", serde_json::json!({ "kind": "node", "n": 1 }));
    let only_hvac = Record::new("rubix", serde_json::json!({ "kind": "node", "n": 2 }));
    create_record(&db, &both).await.expect("create both");
    create_record(&db, &only_hvac).await.expect("create one");

    attach_tag(&db, &both.id, &hvac.id).await.expect("attach");
    attach_tag(&db, &both.id, &floor2.id).await.expect("attach");
    attach_tag(&db, &only_hvac.id, &hvac.id).await.expect("attach");

    // Requiring both tags returns only the record carrying both.
    let matched = list_records_filtered(&db, None, &["hvac".to_owned(), "floor-2".to_owned()])
        .await
        .expect("filter tags");
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id, both.id);

    // Requiring just hvac returns both records.
    let hvac_only = list_records_filtered(&db, None, &["hvac".to_owned()])
        .await
        .expect("filter hvac");
    assert_eq!(hvac_only.len(), 2);
}

#[tokio::test]
async fn kind_and_tag_compose() {
    let db = open_memory_db().await;
    let hvac = Tag::new("hvac");
    create_tag(&db, &hvac).await.expect("tag");

    let site_hvac = Record::new("rubix", serde_json::json!({ "kind": "site", "name": "HQ" }));
    let node_hvac = Record::new("rubix", serde_json::json!({ "kind": "node", "n": 1 }));
    create_record(&db, &site_hvac).await.expect("create site");
    create_record(&db, &node_hvac).await.expect("create node");
    attach_tag(&db, &site_hvac.id, &hvac.id).await.expect("attach");
    attach_tag(&db, &node_hvac.id, &hvac.id).await.expect("attach");

    let matched = list_records_filtered(&db, Some("site"), &["hvac".to_owned()])
        .await
        .expect("compose");
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id, site_hvac.id);
}
