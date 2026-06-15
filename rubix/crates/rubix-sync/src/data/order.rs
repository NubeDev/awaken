//! Apply out-of-order arrivals in a deterministic id/sequence order.
//!
//! The shipper moves append-only data records edge→cloud over Zenoh, which makes
//! no per-key ordering guarantee across a reconnect: a batch can arrive
//! out-of-order, and a replay can interleave re-sent records with fresh ones. The
//! receiver applies them in a deterministic order so two receivers that saw the
//! same set converge to the same applied sequence (`rubix/docs/SCOPE.md`, "Sync
//! and conflict model": reconcile by ordering + dedup). Order is by creation
//! instant, then by id as a stable tiebreak — the id is globally unique, so the
//! total order is total and reproducible regardless of arrival order.

use rubix_core::Record;

/// Return `records` sorted into the deterministic apply order.
///
/// Primary key is the record's `created` instant (append-only data is ordered by
/// when the edge stamped it); the globally-unique id breaks ties so the order is
/// total and independent of arrival order. Does not dedup — that is
/// [`SeenSet`](super::dedup::SeenSet)'s job, applied as records are consumed in
/// this order.
#[must_use]
pub fn in_apply_order(mut records: Vec<Record>) -> Vec<Record> {
    records.sort_by(|a, b| {
        a.created
            .cmp(&b.created)
            .then_with(|| a.id.as_str().cmp(b.id.as_str()))
    });
    records
}

#[cfg(test)]
mod tests {
    use super::in_apply_order;
    use rubix_core::{Id, Record};
    use surrealdb::types::Datetime;

    fn record_at(id: &str, created: Datetime) -> Record {
        Record {
            id: Id::from_raw(id),
            namespace: "edge".to_owned(),
            content: serde_json::json!({}),
            created,
            updated: created,
        }
    }

    #[test]
    fn arrivals_sort_by_created_then_id() {
        let early = Datetime::default();
        let late = Datetime::now();
        // Shuffled input: a late record first, then two early records out of id order.
        let shuffled = vec![
            record_at("z-late", late),
            record_at("b-early", early),
            record_at("a-early", early),
        ];
        let ordered = in_apply_order(shuffled);
        let ids: Vec<&str> = ordered.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["a-early", "b-early", "z-late"]);
    }

    #[test]
    fn ordering_is_stable_regardless_of_arrival_order() {
        let early = Datetime::default();
        let late = Datetime::now();
        let one = vec![record_at("a", early), record_at("b", late)];
        let other = vec![record_at("b", late), record_at("a", early)];
        assert_eq!(in_apply_order(one), in_apply_order(other));
    }
}
