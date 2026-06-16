//! Structured, UTC-correct time scoping for the query surface.
//!
//! The board path sends a structured, UTC [`TimeScope`] — absolute epoch ms or a
//! relative token, plus a grain or a target point count — instead of splicing a
//! locale datetime string into SQL (`rubix/docs/design/DASHBOARDS-SCOPE.md` §5,
//! the timezone bug). The backend resolves the tokens against a single request
//! `now`, snaps the interval ([`scope`]), and rewrites the chart's time macros
//! into real SQL against the resolved UTC window and grain ([`rewrite`]). This is
//! the **one source of truth** for the window and the interval snap; a chart never
//! recomputes the grain table itself.

mod relative;
mod rewrite;
mod scope;

use crate::error::Result;

pub use scope::{ResolvedTimeScope, TimeBound, TimeScope};

/// Apply `scope` to `sql`: resolve its bounds/grain against `now_ms`, then expand
/// the chart's time macros into the resolved UTC window and bucket.
///
/// The returned SQL is the statement the engine runs; it still passes through the
/// read-only guard at the call site (the guard runs on this final string, so a
/// macro can never smuggle a second statement past it). A chart that uses no time
/// macros comes back unchanged but for the resolution being validated.
///
/// # Errors
/// Returns [`QueryError::Rejected`](crate::QueryError::Rejected) if a relative
/// token is unrecognised, the window is inverted, or a bucket/interval macro is
/// used without a grain.
pub fn apply_time_scope(sql: &str, scope: &TimeScope, now_ms: i64) -> Result<String> {
    let resolved = scope.resolve(now_ms)?;
    rewrite::expand_macros(sql, &resolved)
}

/// The current UTC instant in epoch milliseconds — the request-time `now` that
/// relative tokens resolve against.
///
/// Kept here so the transport layer resolves time at one well-named seam rather
/// than reading the clock inline; tests pass an explicit `now_ms` to
/// [`apply_time_scope`] and never touch this.
#[must_use]
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
