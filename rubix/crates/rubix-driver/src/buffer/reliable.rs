//! Bounded reliable queue for the `write` and `his` channels. Unlike the `cur`
//! buffer, a full reliable queue NEVER drops: an enqueue at capacity returns
//! [`DriverError::BufferFull`] so backpressure surfaces to the caller. Items are
//! delivered in FIFO order and only removed once the caller confirms delivery
//! (for `write`, once the ack arrives — see [`super::ack`]).

use std::collections::VecDeque;

use crate::error::DriverError;

/// A bounded FIFO of pending reliable items keyed by the channel they target
/// (used in the [`DriverError::BufferFull`] message). Capacity is the maximum
/// number of un-delivered items; reaching it is backpressure, not loss.
#[derive(Debug)]
pub struct ReliableQueue<T> {
    key: String,
    capacity: usize,
    queue: VecDeque<T>,
}

impl<T> ReliableQueue<T> {
    /// A queue for `key` holding at most `capacity` pending items. `capacity`
    /// is clamped to at least 1 so a reliable channel can always hold one item.
    pub fn new(key: impl Into<String>, capacity: usize) -> Self {
        Self {
            key: key.into(),
            capacity: capacity.max(1),
            queue: VecDeque::new(),
        }
    }

    /// Enqueue `item`, or return [`DriverError::BufferFull`] if the queue is at
    /// capacity. The item is NOT dropped on a full queue — the caller decides
    /// whether to retry, slow down, or fail. This is the reliable contract.
    pub fn enqueue(&mut self, item: T) -> Result<(), DriverError> {
        if self.queue.len() >= self.capacity {
            return Err(DriverError::BufferFull {
                key: self.key.clone(),
                capacity: self.capacity,
            });
        }
        self.queue.push_back(item);
        Ok(())
    }

    /// Inspect the next item to deliver without removing it, so it can be retried
    /// until the caller confirms delivery.
    pub fn front(&self) -> Option<&T> {
        self.queue.front()
    }

    /// Mutable view of the head item, for the caller to update retry bookkeeping
    /// (e.g. an attempt counter) in place between delivery attempts.
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.queue.front_mut()
    }

    /// Remove and return the head item once the caller has confirmed delivery
    /// (e.g. an ack arrived, or the item was given up on).
    pub fn ack_front(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    /// Number of items awaiting delivery.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// True when the queue holds no pending items.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// True when the queue is at capacity (the next enqueue would backpressure).
    pub fn is_full(&self) -> bool {
        self.queue.len() >= self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_delivery_order() {
        let mut q = ReliableQueue::new("nube/hq/ahu-3/temp/write", 4);
        for v in [10, 20, 30] {
            q.enqueue(v).expect("under capacity");
        }
        assert_eq!(q.front(), Some(&10));
        assert_eq!(q.ack_front(), Some(10));
        assert_eq!(q.ack_front(), Some(20));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn full_queue_surfaces_error_never_drops() {
        let mut q = ReliableQueue::new("nube/hq/ahu-3/temp/write", 2);
        q.enqueue(1).unwrap();
        q.enqueue(2).unwrap();
        let err = q.enqueue(3).expect_err("a full reliable queue must refuse");
        assert_eq!(
            err,
            DriverError::BufferFull {
                key: "nube/hq/ahu-3/temp/write".into(),
                capacity: 2,
            }
        );
        // The earlier items are intact — nothing was dropped to admit #3.
        assert_eq!(q.front(), Some(&1));
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn draining_reopens_capacity() {
        let mut q = ReliableQueue::new("k/write", 1);
        q.enqueue(1).unwrap();
        assert!(q.is_full());
        assert!(q.enqueue(2).is_err());
        q.ack_front();
        assert!(q.is_empty());
        q.enqueue(2).expect("capacity reopened after ack");
    }
}
