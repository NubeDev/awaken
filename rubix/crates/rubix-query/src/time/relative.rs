//! Resolve relative time tokens (`now`, `now-1h`, `now/d`) to a UTC instant.
//!
//! A board stores its range as relative tokens so "last 1h" stays fresh across
//! reloads and polls (`rubix/docs/design/DASHBOARDS-SCOPE.md` §5). The tokens are
//! resolved **server-side**, at request time, against a single `now` — never a
//! locale string spliced on the client (the timezone bug this work fixes). The
//! grammar is a deliberately small slice of Grafana's: `now`, an optional
//! `±<count><unit>` offset, and an optional `/<unit>` truncation, all in UTC.
//!
//! Examples (with `now` = 2026-06-16T12:34:56Z):
//! - `now`        → 2026-06-16T12:34:56Z
//! - `now-1h`     → 2026-06-16T11:34:56Z
//! - `now-7d`     → 2026-06-09T12:34:56Z
//! - `now/d`      → 2026-06-16T00:00:00Z (start of today, UTC)
//! - `now-1d/d`   → 2026-06-15T00:00:00Z (start of yesterday)

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};

use crate::error::{QueryError, Result};

/// Resolve a relative token (or a bare epoch-ms integer) to epoch milliseconds.
///
/// `now_ms` is the request-time instant the tokens are relative to — passed in so
/// resolution is deterministic and testable rather than reading the clock here. A
/// purely numeric string is treated as an absolute epoch-ms instant, so a client
/// may send either form on the same field.
///
/// # Errors
/// Returns [`QueryError::Rejected`] if the token is neither a valid epoch-ms
/// integer nor a recognised `now`-relative expression.
pub fn resolve_token(token: &str, now_ms: i64) -> Result<i64> {
    let token = token.trim();
    if let Ok(absolute) = token.parse::<i64>() {
        return Ok(absolute);
    }
    let now = Utc
        .timestamp_millis_opt(now_ms)
        .single()
        .ok_or_else(|| QueryError::Rejected(format!("invalid reference instant: {now_ms}ms")))?;
    resolve_now_expr(token, now).map(|dt| dt.timestamp_millis())
}

/// Parse a `now[±<count><unit>][/<unit>]` expression against `now`.
fn resolve_now_expr(expr: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let rest = expr
        .strip_prefix("now")
        .ok_or_else(|| QueryError::Rejected(format!("not a relative time token: {expr}")))?;

    // Split the optional `/unit` truncation off the tail first.
    let (offset_part, truncate_part) = match rest.split_once('/') {
        Some((head, unit)) => (head, Some(unit)),
        None => (rest, None),
    };

    let mut instant = if offset_part.is_empty() {
        now
    } else {
        apply_offset(offset_part, now)?
    };

    if let Some(unit) = truncate_part {
        instant = truncate(instant, unit)?;
    }
    Ok(instant)
}

/// Apply a `±<count><unit>` offset (e.g. `-1h`, `+30m`) to `now`.
fn apply_offset(part: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
    let sign = match part.as_bytes().first() {
        Some(b'-') => -1,
        Some(b'+') => 1,
        _ => return Err(QueryError::Rejected(format!("expected ± offset, got: {part}"))),
    };
    let body = &part[1..];
    let split = body
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| QueryError::Rejected(format!("offset is missing a unit: {part}")))?;
    let (count_str, unit) = body.split_at(split);
    let count: i64 = count_str
        .parse()
        .map_err(|_| QueryError::Rejected(format!("invalid offset count: {part}")))?;
    let seconds = unit_seconds(unit)?;
    let delta = chrono::Duration::seconds(sign * count * seconds);
    now.checked_add_signed(delta)
        .ok_or_else(|| QueryError::Rejected(format!("offset overflows: {part}")))
}

/// Truncate `instant` to the start of the given calendar unit, in UTC.
fn truncate(instant: DateTime<Utc>, unit: &str) -> Result<DateTime<Utc>> {
    let truncated = match unit {
        "m" => instant.with_second(0).and_then(|t| t.with_nanosecond(0)),
        "h" => instant
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0)),
        "d" => instant
            .with_hour(0)
            .and_then(|t| t.with_minute(0))
            .and_then(|t| t.with_second(0))
            .and_then(|t| t.with_nanosecond(0)),
        "w" => {
            // Truncate to the start of the day, then back up to Monday (ISO week
            // start), matching the rule layer's epoch-aligned week intent.
            let day = instant
                .with_hour(0)
                .and_then(|t| t.with_minute(0))
                .and_then(|t| t.with_second(0))
                .and_then(|t| t.with_nanosecond(0));
            day.map(|d| {
                let back = i64::from(d.weekday().num_days_from_monday());
                d - chrono::Duration::days(back)
            })
        }
        other => return Err(QueryError::Rejected(format!("unknown truncation unit: {other}"))),
    };
    truncated.ok_or_else(|| QueryError::Rejected(format!("could not truncate to {unit}")))
}

/// Seconds in one offset unit (`m`/`h`/`d`/`w`).
fn unit_seconds(unit: &str) -> Result<i64> {
    match unit {
        "m" => Ok(60),
        "h" => Ok(60 * 60),
        "d" => Ok(24 * 60 * 60),
        "w" => Ok(7 * 24 * 60 * 60),
        other => Err(QueryError::Rejected(format!("unknown time unit: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_token;
    use chrono::{TimeZone, Utc};

    // 2026-06-16T12:34:56Z
    fn now() -> i64 {
        Utc.with_ymd_and_hms(2026, 6, 16, 12, 34, 56)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn bare_now_is_the_reference_instant() {
        assert_eq!(resolve_token("now", now()).unwrap(), now());
    }

    #[test]
    fn an_absolute_epoch_ms_passes_through() {
        assert_eq!(resolve_token("1718500000000", now()).unwrap(), 1_718_500_000_000);
    }

    #[test]
    fn negative_hour_offset_subtracts() {
        let got = resolve_token("now-1h", now()).unwrap();
        assert_eq!(got, now() - 3_600_000);
    }

    #[test]
    fn seven_day_offset_subtracts_a_week() {
        let got = resolve_token("now-7d", now()).unwrap();
        assert_eq!(got, now() - 7 * 24 * 3_600_000);
    }

    #[test]
    fn day_truncation_floors_to_utc_midnight() {
        let got = resolve_token("now/d", now()).unwrap();
        let expected = Utc
            .with_ymd_and_hms(2026, 6, 16, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(got, expected);
    }

    #[test]
    fn offset_then_truncation_compose() {
        let got = resolve_token("now-1d/d", now()).unwrap();
        let expected = Utc
            .with_ymd_and_hms(2026, 6, 15, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(got, expected);
    }

    #[test]
    fn week_truncation_floors_to_monday() {
        // 2026-06-16 is a Tuesday; the week start is Monday 2026-06-15.
        let got = resolve_token("now/w", now()).unwrap();
        let expected = Utc
            .with_ymd_and_hms(2026, 6, 15, 0, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        assert_eq!(got, expected);
    }

    #[test]
    fn garbage_tokens_are_rejected() {
        assert!(resolve_token("yesterday", now()).is_err());
        assert!(resolve_token("now-1y", now()).is_err());
        assert!(resolve_token("now*2", now()).is_err());
    }
}
