//! Hard limits applied to a single datasource read.
//!
//! Adapted from nexus `sink/cap.rs`: a read buffers rows in process memory, so
//! an unbounded result would exhaust a node or a browser. Every read carries a
//! [`Caps`]; when admitting the next row would push totals past a limit the
//! collector stops and the result is marked breached. Wall-clock is a separate
//! axis enforced by the backend (Postgres `statement_timeout`), phrased here so
//! a non-Postgres mechanism could slot in later — see [`crate::backend`].

use std::time::Duration;

/// Per-read bounds. A `None` field means "no limit on this axis";
/// [`Caps::unbounded`] disables all three (test/admin use only — production
/// datasource reads always set at least a row and byte cap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// Maximum number of rows to collect before breaching.
    pub max_rows: Option<u64>,
    /// Maximum serialized byte size to collect before breaching.
    pub max_bytes: Option<u64>,
    /// Wall-clock budget for the read, enforced by the backend.
    pub max_duration: Option<Duration>,
}

impl Caps {
    /// No bound on any axis. For tests and trusted internal runs only.
    pub fn unbounded() -> Self {
        Self {
            max_rows: None,
            max_bytes: None,
            max_duration: None,
        }
    }

    /// Cap on row count alone.
    pub fn rows(max_rows: u64) -> Self {
        Self {
            max_rows: Some(max_rows),
            max_bytes: None,
            max_duration: None,
        }
    }

    /// The production default: bound rows, bytes, and wall-clock together.
    pub fn new(max_rows: u64, max_bytes: u64, max_duration: Duration) -> Self {
        Self {
            max_rows: Some(max_rows),
            max_bytes: Some(max_bytes),
            max_duration: Some(max_duration),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_expected_axes() {
        assert_eq!(Caps::unbounded().max_rows, None);
        assert_eq!(Caps::rows(5).max_rows, Some(5));
        assert_eq!(Caps::rows(5).max_bytes, None);
        let full = Caps::new(10, 20, Duration::from_secs(3));
        assert_eq!(full.max_rows, Some(10));
        assert_eq!(full.max_bytes, Some(20));
        assert_eq!(full.max_duration, Some(Duration::from_secs(3)));
    }
}
