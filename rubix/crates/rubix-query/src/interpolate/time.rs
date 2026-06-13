//! The resolved time context the time macros bind against.
//!
//! A dashboard supplies a `{from, to}` range whose bounds are either absolute
//! RFC 3339 instants or relative tokens (`now`, `now-6h`, `now/d`), plus an
//! optional `interval_secs` (docs/design/time-range-and-refresh.md §§1, 4). The
//! server freezes one `now` per request set and resolves the range against it,
//! so every widget in one refresh shares a single instant (no per-widget clock
//! skew — design notes, "Freeze one `now` per refresh"). The resolved
//! [`TimeContext`] is what the macros lower into bound parameters; the raw
//! tokens never reach SQL.

use chrono::{DateTime, Duration, DurationRound, TimeZone, Utc};

use super::error::InterpolateError;

/// A caller-supplied range whose bounds may be relative tokens or absolute
/// instants. Resolved against a frozen `now` into a [`TimeContext`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeRangeSpec {
    /// The lower bound: an RFC 3339 instant or a relative token (`now-6h`).
    pub from: String,
    /// The upper bound: an RFC 3339 instant or a relative token (`now`).
    pub to: String,
    /// The explicit bucket width in seconds, if the caller computed it
    /// client-side. When absent the resolver derives one from the range.
    pub interval_secs: Option<u32>,
}

/// The resolved range the time macros bind against: concrete instants plus the
/// resolved bucket width. Constructed by [`resolve`] from a [`TimeRangeSpec`]
/// and a frozen `now`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeContext {
    /// Inclusive lower bound, resolved to an absolute instant.
    pub from: DateTime<Utc>,
    /// Exclusive upper bound, resolved to an absolute instant.
    pub to: DateTime<Utc>,
    /// The resolved bucket width in seconds (always >= 1).
    pub interval_secs: u32,
}

/// When the caller supplies no `interval_secs`, derive a bucket targeting this
/// many points across the range so `$__timeGroup` yields a sensible series
/// (docs/design/time-range-and-refresh.md design notes, "`$__interval`
/// auto-calculation"). A pixel-accurate target is a client concern (WS-04);
/// this server-side fallback keeps a macro query valid without a client hint.
const DEFAULT_TARGET_POINTS: i64 = 200;

impl TimeContext {
    /// The lower bound as an RFC 3339 string, bound by `$__from` / `$__timeFilter`.
    pub fn lower_rfc3339(&self) -> String {
        self.from.to_rfc3339()
    }

    /// The upper bound as an RFC 3339 string, bound by `$__to` / `$__timeFilter`.
    pub fn upper_rfc3339(&self) -> String {
        self.to.to_rfc3339()
    }
}

/// Resolve a [`TimeRangeSpec`] against a frozen `now` into a [`TimeContext`].
///
/// Each bound is either an absolute RFC 3339 instant or a relative token
/// resolved against `now`. The resolved range must be non-empty (`from < to`).
/// An absent `interval_secs` is derived from the range so a `$__timeGroup`
/// query is always valid.
pub fn resolve(spec: &TimeRangeSpec, now: DateTime<Utc>) -> Result<TimeContext, InterpolateError> {
    let from = resolve_bound(&spec.from, now)?;
    let to = resolve_bound(&spec.to, now)?;
    if from >= to {
        return Err(InterpolateError::EmptyRange {
            from: spec.from.clone(),
            to: spec.to.clone(),
        });
    }
    let interval_secs = match spec.interval_secs {
        Some(secs) if secs >= 1 => secs,
        // 0 or absent: derive a bucket targeting DEFAULT_TARGET_POINTS, clamped
        // to a minimum of one second so the bucket is never degenerate.
        _ => {
            let span = (to - from).num_seconds().max(1);
            (span / DEFAULT_TARGET_POINTS).clamp(1, u32::MAX as i64) as u32
        }
    };
    Ok(TimeContext {
        from,
        to,
        interval_secs,
    })
}

/// Resolve one bound: an absolute RFC 3339 instant or a relative token.
///
/// Relative grammar (a subset of the Grafana-style tokens the picker emits,
/// docs/design/time-range-and-refresh.md §1):
///   - `now`               → the frozen instant.
///   - `now-<n><unit>`     → subtract a duration (`now-6h`, `now-7d`).
///   - `now+<n><unit>`     → add a duration.
///   - `now/<unit>`        → round down to the start of the unit (`now/d`).
///   - `now-<n><unit>/<unit2>` → shift then round.
///
/// Units: `s` `m` `h` `d` `w` (week). Anything else is a parse error rather
/// than a silent fallback that would hide a bad range.
fn resolve_bound(token: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, InterpolateError> {
    let token = token.trim();
    if let Ok(absolute) = DateTime::parse_from_rfc3339(token) {
        return Ok(absolute.with_timezone(&Utc));
    }
    if !token.starts_with("now") {
        return Err(bad_token(token));
    }
    let rest = &token["now".len()..];
    // Split an optional trailing `/<unit>` rounding suffix from the shift.
    let (shift, round) = match rest.split_once('/') {
        Some((shift, round)) => (shift, Some(round)),
        None => (rest, None),
    };
    let mut instant = apply_shift(shift, now)?;
    if let Some(unit) = round {
        instant = round_down(instant, unit)?;
    }
    Ok(instant)
}

/// Apply a `±<n><unit>` shift to `now`. An empty shift leaves `now` unchanged.
fn apply_shift(shift: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, InterpolateError> {
    if shift.is_empty() {
        return Ok(now);
    }
    let (sign, magnitude) = match shift.as_bytes()[0] {
        b'-' => (-1, &shift[1..]),
        b'+' => (1, &shift[1..]),
        _ => return Err(bad_token(shift)),
    };
    let duration = parse_duration(magnitude)?;
    now.checked_add_signed(duration * sign)
        .ok_or_else(|| bad_token(shift))
}

/// Parse `<n><unit>` (`6h`, `7d`, `30m`) into a [`Duration`].
fn parse_duration(text: &str) -> Result<Duration, InterpolateError> {
    let split = text
        .find(|c: char| c.is_ascii_alphabetic())
        .ok_or_else(|| bad_token(text))?;
    let (digits, unit) = text.split_at(split);
    let n: i64 = digits.parse().map_err(|_| bad_token(text))?;
    duration_for(n, unit).ok_or_else(|| bad_token(text))
}

/// Build a [`Duration`] of `n` of `unit`. `None` for an unknown unit.
fn duration_for(n: i64, unit: &str) -> Option<Duration> {
    match unit {
        "s" => Some(Duration::seconds(n)),
        "m" => Some(Duration::minutes(n)),
        "h" => Some(Duration::hours(n)),
        "d" => Some(Duration::days(n)),
        "w" => Some(Duration::weeks(n)),
        _ => None,
    }
}

/// Round `instant` down to the start of `unit` (`now/d` → midnight UTC today).
fn round_down(instant: DateTime<Utc>, unit: &str) -> Result<DateTime<Utc>, InterpolateError> {
    let duration = match unit {
        "s" => Duration::seconds(1),
        "m" => Duration::minutes(1),
        "h" => Duration::hours(1),
        "d" => Duration::days(1),
        // A week floor is anchored to the Unix epoch (a Thursday); the picker
        // owns calendar-week semantics, this is the engine's deterministic floor.
        "w" => Duration::weeks(1),
        _ => return Err(bad_token(unit)),
    };
    instant
        .duration_trunc(duration)
        .map_err(|_| bad_token(unit))
}

/// A relative/absolute token the resolver cannot parse.
fn bad_token(token: &str) -> InterpolateError {
    InterpolateError::BadTimeToken {
        token: token.to_string(),
    }
}

/// The Unix epoch as a UTC instant, a convenience for callers building a spec.
#[allow(dead_code)]
fn epoch() -> DateTime<Utc> {
    Utc.timestamp_opt(0, 0).single().expect("epoch is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        // 2026-06-13T14:30:45Z — a non-aligned instant so rounding is visible.
        Utc.with_ymd_and_hms(2026, 6, 13, 14, 30, 45)
            .single()
            .unwrap()
    }

    fn spec(from: &str, to: &str, interval: Option<u32>) -> TimeRangeSpec {
        TimeRangeSpec {
            from: from.to_string(),
            to: to.to_string(),
            interval_secs: interval,
        }
    }

    #[test]
    fn relative_now_minus_hours_resolves() {
        let ctx = resolve(&spec("now-6h", "now", None), now()).unwrap();
        assert_eq!(ctx.to, now());
        assert_eq!(ctx.from, now() - Duration::hours(6));
    }

    #[test]
    fn now_slash_day_rounds_down_to_midnight() {
        let from = resolve_bound("now/d", now()).unwrap();
        assert_eq!(
            from,
            Utc.with_ymd_and_hms(2026, 6, 13, 0, 0, 0).single().unwrap()
        );
    }

    #[test]
    fn shift_then_round() {
        // now-1d/d → start of yesterday.
        let from = resolve_bound("now-1d/d", now()).unwrap();
        assert_eq!(
            from,
            Utc.with_ymd_and_hms(2026, 6, 12, 0, 0, 0).single().unwrap()
        );
    }

    #[test]
    fn absolute_rfc3339_bounds_resolve() {
        let ctx = resolve(
            &spec("2026-06-13T00:00:00Z", "2026-06-13T06:00:00Z", Some(60)),
            now(),
        )
        .unwrap();
        assert_eq!(ctx.interval_secs, 60);
        assert_eq!(ctx.from.to_rfc3339(), "2026-06-13T00:00:00+00:00");
    }

    #[test]
    fn absent_interval_is_derived_from_range() {
        // 6h span = 21600s; / 200 target = 108s.
        let ctx = resolve(&spec("now-6h", "now", None), now()).unwrap();
        assert_eq!(ctx.interval_secs, 108);
    }

    #[test]
    fn derived_interval_clamps_to_minimum_one_second() {
        // A 10s span / 200 would be 0; clamp to 1.
        let ctx = resolve(&spec("now-10s", "now", None), now()).unwrap();
        assert_eq!(ctx.interval_secs, 1);
    }

    #[test]
    fn zero_interval_is_treated_as_absent_and_derived() {
        let ctx = resolve(&spec("now-6h", "now", Some(0)), now()).unwrap();
        assert_eq!(ctx.interval_secs, 108);
    }

    #[test]
    fn empty_range_is_an_error() {
        let err = resolve(&spec("now", "now-6h", None), now()).unwrap_err();
        assert!(matches!(err, InterpolateError::EmptyRange { .. }));
    }

    #[test]
    fn equal_bounds_are_an_empty_range() {
        let err = resolve(&spec("now", "now", None), now()).unwrap_err();
        assert!(matches!(err, InterpolateError::EmptyRange { .. }));
    }

    #[test]
    fn unknown_unit_is_an_error() {
        let err = resolve_bound("now-6y", now()).unwrap_err();
        assert!(matches!(err, InterpolateError::BadTimeToken { .. }));
    }

    #[test]
    fn garbage_token_is_an_error() {
        let err = resolve_bound("yesterday", now()).unwrap_err();
        assert!(matches!(err, InterpolateError::BadTimeToken { .. }));
    }

    #[test]
    fn injection_in_a_time_token_is_a_parse_error_not_a_bind() {
        // A payload arriving where a token is expected never reaches SQL: it
        // fails to parse as an instant or relative token (the value only ever
        // binds once it is a resolved instant).
        let err = resolve_bound("'); DROP TABLE his; --", now()).unwrap_err();
        assert!(matches!(err, InterpolateError::BadTimeToken { .. }));
    }

    #[test]
    fn epoch_is_unix_zero() {
        assert_eq!(epoch().timestamp(), 0);
    }
}
