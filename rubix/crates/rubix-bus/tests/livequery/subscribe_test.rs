//! Integration: a live-query subscription delivers a data-change on insert.
//!
//! Proves the data-change plane (`rubix/docs/SCOPE.md`, "Event bus"): subscribe
//! to the record table on a gate-issued scoped session, then insert a record in
//! the principal's namespace and assert the subscriber receives a `Created`
//! event carrying that record (verified on kv-mem).

#[path = "../bus/mod.rs"]
mod bus;

use std::time::Duration;

use rubix_bus::{DataChangeKind, subscribe_table};
use rubix_core::{Record, create_record};

use bus::open::{open_bus_store, scoped_session_for};

/// A live-query notification arrives asynchronously; bound the wait so a missing
/// notification fails the test instead of hanging it.
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn insert_pushes_a_created_data_change_to_the_subscriber() {
    let database = "lq_subscribe";
    let handle = open_bus_store(database).await;
    let session = scoped_session_for(&handle, database, "alice", "tenant-a").await;

    let mut stream = subscribe_table(session.connection(), "record")
        .await
        .expect("subscribe to record table");

    // Insert a record in the principal's namespace; the live query should see it.
    let record = Record::new("tenant-a", serde_json::json!({ "temp": 21 }));
    create_record(handle.raw(), &record)
        .await
        .expect("seed record");

    let change = tokio::time::timeout(RECV_TIMEOUT, stream.next())
        .await
        .expect("a notification within the timeout")
        .expect("the stream did not end")
        .expect("the notification decoded");

    assert_eq!(change.kind(), DataChangeKind::Created);
    assert_eq!(change.record().id, record.id);
    assert_eq!(change.record().namespace, "tenant-a");
    assert_eq!(change.record().content, serde_json::json!({ "temp": 21 }));
}
