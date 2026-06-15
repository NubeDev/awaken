//! Integration: collection loading, strict-mode resolution, and bootstrap seeding
//! against a live kv-mem SurrealDB.
//!
//! Exercises the contract layer's read side end to end: a collection record is
//! resolved by its `name`, an unknown kind resolves to `None` (fail-open), the
//! per-namespace strict flag is read from its settings record, and the bootstrap
//! meta-collection seed is idempotent and namespace-scoped.

#[path = "../db/open.rs"]
mod open;

use rubix_core::{
    COLLECTION_KIND, NAMESPACE_SETTINGS_KIND, Record, bootstrap_meta_collection, create_record,
    find_collection, namespace_strict,
};

use open::open_memory_db;

#[tokio::test]
async fn find_collection_resolves_a_definition_by_name() {
    let db = open_memory_db().await;
    let def = Record::new(
        "rubix",
        serde_json::json!({
            "kind": COLLECTION_KIND,
            "name": "site",
            "schema": [{ "name": "key", "type": "text", "required": true }]
        }),
    );
    create_record(&db, &def).await.expect("create collection");

    let found = find_collection(&db, "rubix", "site")
        .await
        .expect("find")
        .expect("present");
    assert_eq!(found.name, "site");
    assert_eq!(found.schema.len(), 1);
}

#[tokio::test]
async fn an_unknown_kind_resolves_to_none() {
    let db = open_memory_db().await;
    let found = find_collection(&db, "rubix", "ghost").await.expect("find");
    assert!(found.is_none());
}

#[tokio::test]
async fn collections_are_namespace_scoped() {
    let db = open_memory_db().await;
    let def = Record::new(
        "tenant-a",
        serde_json::json!({ "kind": COLLECTION_KIND, "name": "site" }),
    );
    create_record(&db, &def).await.expect("create");

    assert!(find_collection(&db, "tenant-a", "site")
        .await
        .expect("find a")
        .is_some());
    assert!(find_collection(&db, "tenant-b", "site")
        .await
        .expect("find b")
        .is_none());
}

#[tokio::test]
async fn namespace_strict_defaults_false_and_reads_the_settings_record() {
    let db = open_memory_db().await;
    assert!(!namespace_strict(&db, "rubix").await.expect("default"));

    let settings = Record::new(
        "rubix",
        serde_json::json!({ "kind": NAMESPACE_SETTINGS_KIND, "strict": true }),
    );
    create_record(&db, &settings).await.expect("create settings");
    assert!(namespace_strict(&db, "rubix").await.expect("strict on"));
}

#[tokio::test]
async fn bootstrap_meta_collection_is_idempotent_and_scoped() {
    let db = open_memory_db().await;

    bootstrap_meta_collection(&db, "rubix").await.expect("seed 1");
    bootstrap_meta_collection(&db, "rubix").await.expect("seed 2");

    // Exactly one meta-collection exists for the namespace after two seeds.
    let count: Option<i64> = db
        .query(
            "SELECT VALUE count() FROM record \
             WHERE namespace = 'rubix' AND content.kind = 'collection' \
               AND content.name = 'collection' GROUP ALL",
        )
        .await
        .expect("count")
        .take(0)
        .expect("decode");
    assert_eq!(count, Some(1));

    // The meta-collection is resolvable as a real definition.
    let meta = find_collection(&db, "rubix", "collection")
        .await
        .expect("find")
        .expect("present");
    assert_eq!(meta.name, "collection");

    // Another namespace is untouched until seeded.
    assert!(find_collection(&db, "other", "collection")
        .await
        .expect("find other")
        .is_none());
}
