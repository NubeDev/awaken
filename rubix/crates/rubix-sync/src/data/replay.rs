//! Re-send unacked records on reconnect, idempotently.
//!
//! The shipper is at-least-once over Zenoh: a record published before a
//! disconnect may or may not have landed, so on reconnect the edge re-sends every
//! record the cloud has not acked. Combined with receiver-side dedup
//! ([`SeenSet`](super::dedup::SeenSet)) this is exactly-once *effect* — a re-sent
//! record that already landed is a no-op (`rubix/docs/SCOPE.md`, "Sync and
//! conflict model": idempotent replay). This module owns the edge-side accounting:
//! which ids are still in flight, and which to re-send when the link returns.

use std::collections::HashMap;

use rubix_core::{Id, Record};

/// The edge-side record of what has been shipped but not yet acked.
///
/// A record enters the outbox when it is shipped ([`enqueue`](Outbox::enqueue))
/// and leaves when the cloud acks it ([`ack`](Outbox::ack)). On reconnect the
/// edge re-ships exactly the still-unacked set ([`unacked`](Outbox::unacked)).
/// Keyed by id so re-enqueuing the same record (e.g. it was re-shipped before its
/// ack arrived) does not double-count — the outbox holds at most one entry per id.
#[derive(Debug, Default, Clone)]
pub struct Outbox {
    pending: HashMap<Id, Record>,
}

impl Outbox {
    /// An empty outbox.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `record` has been shipped and is awaiting an ack.
    ///
    /// Idempotent in the id: re-enqueuing the same id replaces the entry rather
    /// than adding a duplicate, so the unacked set never holds two copies of one
    /// record.
    pub fn enqueue(&mut self, record: Record) {
        self.pending.insert(record.id.clone(), record);
    }

    /// Drop `id` from the outbox because the cloud acked it.
    ///
    /// Returns `true` if the id was pending, `false` if it was already acked (a
    /// duplicate ack is harmless).
    pub fn ack(&mut self, id: &Id) -> bool {
        self.pending.remove(id).is_some()
    }

    /// The records still awaiting an ack, in id order — the set to re-ship on
    /// reconnect.
    ///
    /// Sorted by id so the re-ship order is deterministic regardless of the map's
    /// internal iteration order; the receiver re-orders by `created` on apply
    /// anyway ([`in_apply_order`](super::order::in_apply_order)), so this is only
    /// for a stable, testable wire order.
    #[must_use]
    pub fn unacked(&self) -> Vec<Record> {
        let mut records: Vec<Record> = self.pending.values().cloned().collect();
        records.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        records
    }

    /// How many records are still awaiting an ack.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// `true` when every shipped record has been acked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::Outbox;
    use rubix_core::{Id, Record};

    fn record(id: &str) -> Record {
        let mut r = Record::new("edge", serde_json::json!({ "id": id }));
        r.id = Id::from_raw(id);
        r
    }

    #[test]
    fn unacked_holds_what_was_enqueued_until_acked() {
        let mut outbox = Outbox::new();
        outbox.enqueue(record("a"));
        outbox.enqueue(record("b"));
        assert_eq!(outbox.len(), 2);

        assert!(outbox.ack(&Id::from_raw("a")));
        let pending = outbox.unacked();
        let unacked: Vec<&str> = pending.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(unacked, vec!["b"], "only the still-pending record re-ships");
    }

    #[test]
    fn re_enqueuing_an_id_does_not_double_count() {
        let mut outbox = Outbox::new();
        outbox.enqueue(record("a"));
        outbox.enqueue(record("a"));
        assert_eq!(outbox.len(), 1, "the outbox holds one entry per id");
    }

    #[test]
    fn acking_an_unknown_id_is_harmless() {
        let mut outbox = Outbox::new();
        assert!(!outbox.ack(&Id::from_raw("ghost")));
        assert!(outbox.is_empty());
    }
}
