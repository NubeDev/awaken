//! Running totals checked against [`Caps`] as rows arrive.
//!
//! Adapted from nexus `CapState`. `admit` decides whether the next row fits and
//! records a breach so the executor can either truncate (dashboard path) or
//! error (spark path) — the policy is the caller's, not baked in here (see
//! [`crate::executor`]).

use super::limit::Caps;

/// Accumulated row/byte totals for one read, with a sticky breach flag.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CapState {
    pub rows: u64,
    pub bytes: u64,
    pub breached: bool,
}

impl CapState {
    /// Account for one row of `bytes`. Returns `true` if it fits within the caps
    /// and was admitted; `false` if it would breach a limit — in which case
    /// `breached` is set and the row must be dropped and collection stopped. A
    /// row that would cross a limit is rejected whole, so totals never exceed
    /// the cap.
    pub fn admit(&mut self, bytes: u64, caps: &Caps) -> bool {
        if let Some(max) = caps.max_rows {
            if self.rows + 1 > max {
                self.breached = true;
                return false;
            }
        }
        if let Some(max) = caps.max_bytes {
            if self.bytes + bytes > max {
                self.breached = true;
                return false;
            }
        }
        self.rows += 1;
        self.bytes += bytes;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_until_row_cap_then_breaches() {
        let caps = Caps::rows(2);
        let mut state = CapState::default();
        assert!(state.admit(100, &caps));
        assert!(state.admit(100, &caps));
        assert!(!state.admit(100, &caps), "third row exceeds row cap of 2");
        assert!(state.breached);
        assert_eq!(state.rows, 2, "a rejected row does not count");
    }

    #[test]
    fn byte_cap_is_independent_of_row_cap() {
        let caps = Caps {
            max_rows: None,
            max_bytes: Some(50),
            max_duration: None,
        };
        let mut state = CapState::default();
        assert!(state.admit(40, &caps));
        assert!(!state.admit(20, &caps), "40 + 20 > 50");
        assert!(state.breached);
        assert_eq!(state.bytes, 40);
    }

    #[test]
    fn unbounded_never_breaches() {
        let caps = Caps::unbounded();
        let mut state = CapState::default();
        for _ in 0..1000 {
            assert!(state.admit(10_000, &caps));
        }
        assert!(!state.breached);
    }
}
