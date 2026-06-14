//! Integration: the bootstrap step points the connection at the configured
//! namespace/database, isolating data written under different databases.

use rubix_core::RuntimeConfig;
use rubix_store::StoreHandle;
use surrealdb::types::SurrealValue;

#[derive(Debug, Clone, PartialEq, SurrealValue)]
struct Marker {
    note: String,
}

#[tokio::test]
async fn each_database_isolates_its_own_records() {
    // Two handles over the same in-memory namespace but different databases.
    // Each writes the same table/id with different content; neither must observe
    // the other's value — confirming bootstrap selects the configured database.
    let first = StoreHandle::open(&RuntimeConfig::in_memory("rubix", "db_a"))
        .await
        .expect("open db_a");
    first
        .create("marker", "shared-id", Marker { note: "a".into() })
        .await
        .expect("write to db_a");

    let second = StoreHandle::open(&RuntimeConfig::in_memory("rubix", "db_b"))
        .await
        .expect("open db_b");
    second
        .create("marker", "shared-id", Marker { note: "b".into() })
        .await
        .expect("write to db_b");

    let from_a: Marker = first
        .read("marker", "shared-id")
        .await
        .expect("read db_a")
        .expect("db_a record present");
    let from_b: Marker = second
        .read("marker", "shared-id")
        .await
        .expect("read db_b")
        .expect("db_b record present");

    assert_eq!(from_a.note, "a");
    assert_eq!(from_b.note, "b");
}
