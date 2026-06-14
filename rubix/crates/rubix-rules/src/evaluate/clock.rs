//! The wall-clock read used to stamp span start/end timestamps.
//!
//! A [`Span`](rubix_trace::Span) carries caller-supplied epoch-nanosecond
//! timestamps so the span model stays free of an I/O dependency (`rubix-trace`).
//! The evaluation pipeline is that caller: it reads the clock once at the start
//! and once at the end of each rule's evaluation to bound the span. This is the
//! one place the rules crate touches a wall clock, named so it is obvious and
//! easy to find rather than scattered as inline `SystemTime` calls.

use std::time::{SystemTime, UNIX_EPOCH};

/// The current time in epoch nanoseconds.
///
/// A pre-epoch clock (a misconfigured device before 1970) floors to `0` rather
/// than wrapping negative, so span timestamps stay monotonic-comparable; the
/// nanosecond count is saturated into `i64` to match the span field type.
#[must_use]
pub fn now_ns() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => i64::try_from(elapsed.as_nanos()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::now_ns;

    #[test]
    fn the_clock_reads_a_positive_recent_instant() {
        // After 2020-01-01 in nanoseconds — proves the clock is wired to epoch.
        assert!(now_ns() > 1_577_836_800_000_000_000);
    }
}
