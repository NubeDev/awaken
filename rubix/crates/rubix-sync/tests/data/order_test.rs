//! Integration: out-of-order arrivals apply in deterministic id/sequence order.
//!
//! Zenoh makes no cross-reconnect ordering guarantee, so a batch can arrive
//! out-of-order (`rubix/docs/sessions/WS-15.md`). The receiver applies in a
//! deterministic order (by `created`, then id), so two receivers that saw the same
//! set converge to the same applied sequence. This test delivers a shuffled batch
//! and asserts every record lands, then asserts the apply order is independent of
//! arrival order by replaying the same set in a different shuffle.

#[path = "../fixture/mod.rs"]
mod fixture;

use fixture::{data_record, open_store};
use rubix_core::{Id, read_record};
use rubix_sync::{SeenSet, in_apply_order, apply_batch};
use surrealdb::types::Datetime;

#[tokio::test]
async fn a_shuffled_batch_lands_every_record() {
    let cloud = open_store("order_lands").await;

    // Three records with distinct creation instants, delivered out of order.
    let early = Datetime::default();
    let mid = Datetime::now();
    let mut a = data_record("a", serde_json::json!({ "seq": 0 }));
    a.created = early;
    let mut b = data_record("b", serde_json::json!({ "seq": 1 }));
    b.created = mid;
    let mut c = data_record("c", serde_json::json!({ "seq": 2 }));
    c.created = mid; // same instant as b — id breaks the tie deterministically

    let shuffled = vec![c.clone(), a.clone(), b.clone()];
    let mut seen = SeenSet::new();
    let applied = apply_batch(cloud.raw(), &mut seen, shuffled)
        .await
        .expect("apply shuffled batch");
    assert_eq!(applied, 3, "every record in the shuffled batch lands");

    for id in ["a", "b", "c"] {
        assert!(
            read_record(cloud.raw(), &Id::from_raw(id))
                .await
                .expect("read")
                .is_some(),
            "record {id} landed",
        );
    }
}

#[tokio::test]
async fn apply_order_is_independent_of_arrival_order() {
    let early = Datetime::default();
    let late = Datetime::now();
    let mut a = data_record("a", serde_json::json!({}));
    a.created = early;
    let mut b = data_record("b", serde_json::json!({}));
    b.created = late;

    // Two different arrival orders must produce the same apply order.
    let one = in_apply_order(vec![a.clone(), b.clone()]);
    let other = in_apply_order(vec![b, a]);
    let ids_one: Vec<&str> = one.iter().map(|r| r.id.as_str()).collect();
    let ids_other: Vec<&str> = other.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids_one, ids_other);
    assert_eq!(ids_one, vec!["a", "b"], "earlier creation applies first");
}
