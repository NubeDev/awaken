//! Integration: data records ship edge→cloud and replay dedups idempotently.
//!
//! The data plane is append-only and edge-partitioned (`rubix/docs/sessions/
//! WS-15.md`, contract #5): records ship from the edge to the cloud, and a replay
//! that re-sends the same batch on reconnect is an idempotent no-op — every record
//! lands exactly once. This test ships a batch into the cloud store, asserts each
//! landed once, then replays the *same* batch through the same receiver state and
//! asserts no duplicates and no mutation.

#[path = "../fixture/mod.rs"]
mod fixture;

use fixture::{data_record, open_store};
use rubix_core::read_record;
use rubix_sync::{Outbox, SeenSet, apply_batch};

#[tokio::test]
async fn a_replayed_batch_lands_each_record_exactly_once() {
    let cloud = open_store("ship_dedup_cloud").await;

    // The edge ships three append-only records; the outbox tracks them as unacked.
    let mut outbox = Outbox::new();
    for n in 0..3 {
        outbox.enqueue(data_record(&format!("rec-{n}"), serde_json::json!({ "temp": 20 + n })));
    }
    let batch = outbox.unacked();
    assert_eq!(batch.len(), 3);

    // First delivery: the cloud applies the batch into its store, deduping by id.
    let mut seen = SeenSet::new();
    let applied = apply_batch(cloud.raw(), &mut seen, batch.clone())
        .await
        .expect("apply first delivery");
    assert_eq!(applied, 3, "all three records land on first delivery");
    assert_eq!(seen.len(), 3);

    // The records are present in the cloud store under their edge ids.
    for n in 0..3 {
        let id = rubix_core::Id::from_raw(format!("rec-{n}"));
        let landed = read_record(cloud.raw(), &id)
            .await
            .expect("read landed record")
            .expect("record present in cloud");
        assert_eq!(landed.content, serde_json::json!({ "temp": 20 + n }));
    }

    // Reconnect: the edge re-ships the same unacked batch. With receiver dedup this
    // is a no-op — nothing is applied a second time.
    let replayed = apply_batch(cloud.raw(), &mut seen, batch.clone())
        .await
        .expect("apply replay");
    assert_eq!(replayed, 0, "a replayed batch applies nothing new");
    assert_eq!(seen.len(), 3, "dedup keeps exactly one entry per id");

    // The cloud rows are unchanged after the replay — no mutation, no duplication.
    for record in &batch {
        let landed = read_record(cloud.raw(), &record.id)
            .await
            .expect("read after replay")
            .expect("record still present");
        assert_eq!(&landed, record, "replay did not mutate the landed record");
    }
}

#[tokio::test]
async fn a_fresh_receiver_skips_records_already_in_the_store() {
    // Idempotency must survive a receiver restart: a fresh `SeenSet` (empty) that
    // re-applies a record already in the store must not re-insert it.
    let cloud = open_store("ship_dedup_restart").await;
    let record = data_record("rec-1", serde_json::json!({ "v": 1 }));

    let mut seen = SeenSet::new();
    let applied = apply_batch(cloud.raw(), &mut seen, vec![record.clone()])
        .await
        .expect("first apply");
    assert_eq!(applied, 1);

    // Simulate a receiver restart: brand-new in-memory dedup state.
    let mut restarted = SeenSet::new();
    let after_restart = apply_batch(cloud.raw(), &mut restarted, vec![record.clone()])
        .await
        .expect("apply after restart");
    assert_eq!(after_restart, 0, "the store is the durable dedup across restarts");
}
