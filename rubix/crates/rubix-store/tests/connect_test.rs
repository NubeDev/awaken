//! Integration: opening the in-memory engine and a record write→read round-trip
//! through the durable boundary handle.

use rubix_core::RuntimeConfig;
use rubix_store::StoreHandle;
use surrealdb::types::SurrealValue;

#[derive(Debug, Clone, PartialEq, SurrealValue)]
struct Reading {
    label: String,
    value: i64,
}

#[tokio::test]
async fn open_in_memory_engine_succeeds() {
    let cfg = RuntimeConfig::in_memory("rubix", "main");
    let handle = StoreHandle::open(&cfg).await.expect("open in-memory store");
    handle.health().await.expect("fresh handle is healthy");
}

#[tokio::test]
async fn write_then_read_round_trips_the_stored_value() {
    let cfg = RuntimeConfig::in_memory("rubix", "round_trip");
    let handle = StoreHandle::open(&cfg).await.expect("open store");

    let stored = Reading {
        label: "temp".into(),
        value: 21,
    };
    let created = handle
        .create("reading", "r1", stored.clone())
        .await
        .expect("create record")
        .expect("created record returned");
    assert_eq!(created, stored);

    let read: Reading = handle
        .read("reading", "r1")
        .await
        .expect("read record")
        .expect("record present");
    assert_eq!(read, stored);
}

#[tokio::test]
async fn reading_a_missing_record_returns_none() {
    let cfg = RuntimeConfig::in_memory("rubix", "missing");
    let handle = StoreHandle::open(&cfg).await.expect("open store");
    // Materialise the table with one record so the absent-id read resolves to
    // `None` rather than a table-not-found error.
    handle
        .create(
            "reading",
            "present",
            Reading {
                label: "seed".into(),
                value: 0,
            },
        )
        .await
        .expect("seed record");
    let read: Option<Reading> = handle.read("reading", "absent").await.expect("read");
    assert!(read.is_none());
}
