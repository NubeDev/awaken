//! Integration: the data-change plane is scope-filtered once at subscribe.
//!
//! Proves contract #1 (`rubix/STACK-DEISGN.md`): a principal subscribed on its
//! scoped session receives changes only to records its row-level permissions
//! allow. A record written in a foreign namespace produces no event for it; the
//! filter is the engine's, applied at the live query, not an app proxy per
//! message.
//!
//! The assertion is ordering-based: after the foreign write, an own-namespace
//! write follows. If the subscriber's first delivered event is the own-namespace
//! record, the foreign record was never delivered — a clean negative without a
//! flaky "wait and hope nothing arrives".

#[path = "../bus/mod.rs"]
mod bus;

use std::time::Duration;

use rubix_bus::{DataChangeKind, subscribe_table};
use rubix_core::{Record, create_record};

use bus::open::{open_bus_store, scoped_session_for};

const RECV_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn a_foreign_namespace_change_is_not_delivered() {
    let database = "lq_scope_filter";
    let handle = open_bus_store(database).await;
    // The subscriber's principal is scoped to tenant-a.
    let session = scoped_session_for(&handle, database, "alice", "tenant-a").await;

    let mut stream = subscribe_table(session.connection(), "record")
        .await
        .expect("subscribe to record table");

    // A change in tenant-b must be invisible to a tenant-a subscriber.
    let foreign = Record::new("tenant-b", serde_json::json!({ "secret": true }));
    create_record(handle.raw(), &foreign)
        .await
        .expect("seed foreign record");

    // A change in the subscriber's own namespace must be delivered.
    let own = Record::new("tenant-a", serde_json::json!({ "temp": 21 }));
    create_record(handle.raw(), &own)
        .await
        .expect("seed own record");

    let change = tokio::time::timeout(RECV_TIMEOUT, stream.next())
        .await
        .expect("a notification within the timeout")
        .expect("the stream did not end")
        .expect("the notification decoded");

    assert_eq!(change.kind(), DataChangeKind::Created);
    assert_eq!(
        change.record().id,
        own.id,
        "the first delivered change is the own-namespace record; the foreign one was filtered out",
    );
    assert_eq!(change.record().namespace, "tenant-a");
}
