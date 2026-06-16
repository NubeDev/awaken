//! The structured, UTC time scope a query carries, and its resolved form.
//!
//! Time is a structured backend concern (`rubix/docs/design/DASHBOARDS-SCOPE.md`
//! §5): a request sends absolute UTC epoch milliseconds or a relative token, plus
//! either an explicit grain or a target point count — never a locale-formatted
//! string. [`TimeScope`] is the unresolved request shape; [`ResolvedTimeScope`]
//! is what the macro rewrite consumes after the backend has resolved the tokens
//! to instants and snapped the interval. The backend owns both resolutions, so a
//! chart never recomputes the grain table.

use crate::aggregate::Grain;
use crate::error::{QueryError, Result};

use super::relative::resolve_token;

/// One end of a time window: an absolute epoch-ms instant or a relative token.
///
/// A board stores relative tokens (`now`, `now-1h`) so a "last 1h" range stays
/// fresh across reloads; a console may send an absolute instant. Both resolve to
/// epoch milliseconds against the request-time `now`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeBound {
    /// An absolute UTC instant in epoch milliseconds.
    Absolute(i64),
    /// A relative token (`now`, `now-7d`, `now/d`) resolved server-side.
    Relative(String),
}

impl TimeBound {
    /// Resolve this bound to epoch milliseconds against `now_ms`.
    ///
    /// # Errors
    /// Returns [`QueryError::Rejected`] if a relative token is unrecognised.
    pub fn resolve(&self, now_ms: i64) -> Result<i64> {
        match self {
            TimeBound::Absolute(ms) => Ok(*ms),
            TimeBound::Relative(token) => resolve_token(token, now_ms),
        }
    }
}

/// The unresolved time scope a query request carries.
///
/// `from`/`to` are window bounds; `grain` pins an explicit bucket width and
/// `target_points` asks the backend to snap to whichever grain yields about that
/// many buckets. An explicit `grain` wins over `target_points`; with neither, the
/// window is applied as a filter but no bucket macro may be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeScope {
    /// The inclusive lower bound of the window.
    pub from: TimeBound,
    /// The inclusive upper bound of the window.
    pub to: TimeBound,
    /// An explicit bucket grain, if the client pinned one.
    pub grain: Option<Grain>,
    /// A desired bucket count the backend snaps a grain to (ignored if `grain` is
    /// set).
    pub target_points: Option<u32>,
}

impl TimeScope {
    /// Resolve the bounds against `now_ms` and snap the grain — the single place
    /// the backend turns a request scope into the form the rewrite consumes.
    ///
    /// `from` must not be after `to`. If `grain` is set it is used as-is; else if
    /// `target_points` is set the grain is snapped to it
    /// ([`Grain::for_target_points`]); else the resolved scope has no grain (a
    /// window filter only).
    ///
    /// # Errors
    /// Returns [`QueryError::Rejected`] if a token is unrecognised or the window
    /// is inverted (`from` after `to`).
    pub fn resolve(&self, now_ms: i64) -> Result<ResolvedTimeScope> {
        let from_ms = self.from.resolve(now_ms)?;
        let to_ms = self.to.resolve(now_ms)?;
        if from_ms > to_ms {
            return Err(QueryError::Rejected(format!(
                "time window is inverted: from {from_ms}ms is after to {to_ms}ms"
            )));
        }
        let from_micros = from_ms.saturating_mul(1_000);
        let to_micros = to_ms.saturating_mul(1_000);
        let grain = match self.grain {
            Some(grain) => Some(grain),
            None => self
                .target_points
                .map(|target| Grain::for_target_points(to_micros - from_micros, target)),
        };
        Ok(ResolvedTimeScope::new(from_micros, to_micros, grain))
    }
}

/// A time scope with bounds resolved to epoch micros and the grain snapped.
///
/// This is what [`expand_macros`](super::rewrite::expand_macros) reads. Bounds are
/// in **microseconds** to match the canonical `created` column's resolution
/// (`Timestamp(Microsecond, None)`), so the rewritten filter compares like with
/// like and the bucket floor reuses [`Grain::bucket_start`]'s alignment exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedTimeScope {
    from_micros: i64,
    to_micros: i64,
    grain: Option<Grain>,
}

impl ResolvedTimeScope {
    /// Build a resolved scope from epoch-microsecond bounds and an optional grain.
    #[must_use]
    pub fn new(from_micros: i64, to_micros: i64, grain: Option<Grain>) -> Self {
        Self {
            from_micros,
            to_micros,
            grain,
        }
    }

    /// The window's inclusive lower bound, in epoch microseconds.
    #[must_use]
    pub fn from_micros(&self) -> i64 {
        self.from_micros
    }

    /// The window's inclusive upper bound, in epoch microseconds.
    #[must_use]
    pub fn to_micros(&self) -> i64 {
        self.to_micros
    }

    /// The snapped bucket grain, if one was requested.
    #[must_use]
    pub fn grain(&self) -> Option<Grain> {
        self.grain
    }
}

#[cfg(test)]
mod tests {
    use super::{TimeBound, TimeScope};
    use crate::aggregate::Grain;

    const HOUR_MS: i64 = 3_600_000;

    fn now() -> i64 {
        1_750_000_000_000
    }

    #[test]
    fn absolute_bounds_convert_to_micros() {
        let scope = TimeScope {
            from: TimeBound::Absolute(now() - HOUR_MS),
            to: TimeBound::Absolute(now()),
            grain: None,
            target_points: None,
        };
        let resolved = scope.resolve(now()).unwrap();
        assert_eq!(resolved.from_micros(), (now() - HOUR_MS) * 1_000);
        assert_eq!(resolved.to_micros(), now() * 1_000);
        assert_eq!(resolved.grain(), None);
    }

    #[test]
    fn relative_tokens_resolve_against_now() {
        let scope = TimeScope {
            from: TimeBound::Relative("now-1h".to_owned()),
            to: TimeBound::Relative("now".to_owned()),
            grain: Some(Grain::Minute),
            target_points: None,
        };
        let resolved = scope.resolve(now()).unwrap();
        assert_eq!(resolved.from_micros(), (now() - HOUR_MS) * 1_000);
        assert_eq!(resolved.to_micros(), now() * 1_000);
        assert_eq!(resolved.grain(), Some(Grain::Minute));
    }

    #[test]
    fn target_points_snaps_a_grain_when_none_is_pinned() {
        let scope = TimeScope {
            from: TimeBound::Absolute(now() - 24 * HOUR_MS),
            to: TimeBound::Absolute(now()),
            grain: None,
            target_points: Some(24),
        };
        let resolved = scope.resolve(now()).unwrap();
        assert_eq!(resolved.grain(), Some(Grain::Hour));
    }

    #[test]
    fn explicit_grain_wins_over_target_points() {
        let scope = TimeScope {
            from: TimeBound::Absolute(now() - 24 * HOUR_MS),
            to: TimeBound::Absolute(now()),
            grain: Some(Grain::Day),
            target_points: Some(1000),
        };
        let resolved = scope.resolve(now()).unwrap();
        assert_eq!(resolved.grain(), Some(Grain::Day));
    }

    #[test]
    fn an_inverted_window_is_rejected() {
        let scope = TimeScope {
            from: TimeBound::Absolute(now()),
            to: TimeBound::Absolute(now() - HOUR_MS),
            grain: None,
            target_points: None,
        };
        assert!(scope.resolve(now()).is_err());
    }
}
