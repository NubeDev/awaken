//! Bounded latest-wins buffer for the live `cur` channel. Under pressure the
//! oldest queued sample is dropped to admit the newest — the opposite of the
//! reliable channels — because for a live value only the freshest reading
//! matters. Every drop increments a counter so truncation is observable (the
//! "`log()`-visible counter of dropped `cur` samples" in WS-10), never silent.

use std::collections::VecDeque;

/// A bounded ring of pending `cur` samples with a drop-oldest overflow policy.
/// Generic over the sample payload so the driver decides the encoding; the
/// buffer only owns ordering, the bound, and the dropped-sample counter.
#[derive(Debug)]
pub struct CurBuffer<T> {
    capacity: usize,
    queue: VecDeque<T>,
    dropped: u64,
}

impl<T> CurBuffer<T> {
    /// A buffer holding at most `capacity` pending samples. `capacity` must be
    /// at least 1; a zero-capacity live buffer could never deliver a value, so
    /// it is clamped to 1 rather than silently accepting nothing.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            queue: VecDeque::new(),
            dropped: 0,
        }
    }

    /// Enqueue `sample`. If the buffer is full, drop the oldest sample first and
    /// count it. Returns the dropped sample (if any) so a caller that wants to
    /// log the specific value can; the counter is bumped regardless.
    pub fn push(&mut self, sample: T) -> Option<T> {
        let evicted = if self.queue.len() >= self.capacity {
            self.dropped += 1;
            self.queue.pop_front()
        } else {
            None
        };
        self.queue.push_back(sample);
        evicted
    }

    /// Take the next sample to publish in arrival order, or `None` if empty.
    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    /// Number of samples currently queued.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// True when no samples are queued.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Total samples dropped to overflow since construction. Exposed so the
    /// driver can `log()` it (per WS-10) — drops are counted, never hidden.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_capacity_keeps_everything_in_order() {
        let mut b = CurBuffer::new(4);
        for v in [1, 2, 3] {
            assert!(b.push(v).is_none());
        }
        assert_eq!(b.dropped(), 0);
        assert_eq!(b.pop(), Some(1));
        assert_eq!(b.pop(), Some(2));
        assert_eq!(b.pop(), Some(3));
        assert!(b.is_empty());
    }

    #[test]
    fn flooded_drops_oldest_and_newest_survive() {
        let mut b = CurBuffer::new(3);
        // Push 6 into a 3-slot buffer; the first 3 should be evicted.
        let mut evicted = Vec::new();
        for v in 1..=6 {
            if let Some(old) = b.push(v) {
                evicted.push(old);
            }
        }
        assert_eq!(evicted, vec![1, 2, 3]);
        assert_eq!(b.dropped(), 3, "drops are counted, not silent");
        // The three newest survived, in order.
        assert_eq!(b.pop(), Some(4));
        assert_eq!(b.pop(), Some(5));
        assert_eq!(b.pop(), Some(6));
        assert!(b.is_empty());
    }

    #[test]
    fn zero_capacity_is_clamped_to_one() {
        let mut b = CurBuffer::new(0);
        assert!(b.push(10).is_none());
        assert_eq!(b.push(20), Some(10));
        assert_eq!(b.dropped(), 1);
        assert_eq!(b.pop(), Some(20));
    }
}
