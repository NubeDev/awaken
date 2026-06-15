//! Drop already-seen ids so a replayed record is an idempotent no-op.
//!
//! The data plane is append-only and edge-partitioned (`rubix/STACK-DEISGN.md`,
//! contract #5): two edges never mint the same id, so reconciliation at the
//! receiver is dedup by id, never a merge. A record can arrive more than once — a
//! reconnect re-sends everything unacked ([`replay`](super::replay)) — so the
//! receiver must treat a re-sent id as a no-op rather than a second insert. This
//! is the one place that decision is taken: an id is applied at most once.

use std::collections::HashSet;

use rubix_core::Id;

/// The set of record ids already applied at the receiver.
///
/// Seeded empty and grown as records land. [`is_new`](SeenSet::is_new) reports
/// whether an id has not yet been applied; [`mark`](SeenSet::mark) records that it
/// now has. Keyed on the edge-mintable [`Id`], which is globally unique without
/// coordination, so an id collision can only be the same record re-sent — exactly
/// the case dedup must absorb.
#[derive(Debug, Default, Clone)]
pub struct SeenSet {
    seen: HashSet<Id>,
}

impl SeenSet {
    /// An empty seen-set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` when `id` has not yet been applied at this receiver.
    #[must_use]
    pub fn is_new(&self, id: &Id) -> bool {
        !self.seen.contains(id)
    }

    /// Record that `id` has now been applied.
    ///
    /// Returns `true` if this marked a genuinely new id, `false` if the id was
    /// already present (a re-sent record). The boolean lets a caller count
    /// genuine applies without a second lookup.
    pub fn mark(&mut self, id: Id) -> bool {
        self.seen.insert(id)
    }

    /// How many distinct ids have been applied.
    #[must_use]
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// `true` when no id has been applied yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::SeenSet;
    use rubix_core::Id;

    #[test]
    fn an_unseen_id_is_new_then_marking_makes_it_seen() {
        let mut seen = SeenSet::new();
        let id = Id::from_raw("rec-1");
        assert!(seen.is_new(&id));
        assert!(seen.mark(id.clone()), "first mark is a genuine apply");
        assert!(!seen.is_new(&id));
        assert_eq!(seen.len(), 1);
    }

    #[test]
    fn re_marking_a_seen_id_is_a_no_op() {
        let mut seen = SeenSet::new();
        let id = Id::from_raw("rec-1");
        assert!(seen.mark(id.clone()));
        assert!(!seen.mark(id.clone()), "a re-sent id does not count again");
        assert_eq!(seen.len(), 1, "dedup keeps exactly one entry per id");
    }
}
