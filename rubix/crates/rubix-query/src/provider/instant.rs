//! Parse a SurrealDB RFC3339 datetime into epoch microseconds.
//!
//! SurrealDB serialises a stored `datetime` to a UTC RFC3339 string in JSON. The
//! Arrow timestamp columns and the window-bucket math (`crate::aggregate`) both
//! work in epoch microseconds, so the scan and the series reader share this one
//! parser rather than each carrying its own date handling (`docs/FILE-LAYOUT.md`,
//! dedup). It is intentionally dependency-free: stored datetimes are always UTC
//! RFC3339, so a fixed-shape parser is sufficient and avoids pulling a date
//! library in for one field.

/// Parse a SurrealDB `created`/`updated` RFC3339 string into epoch microseconds.
///
/// Returns `None` for any string that is not RFC3339 UTC of the expected shape
/// rather than guessing — a column that fails to parse stays null instead of
/// landing at a wrong instant.
#[must_use]
pub fn parse_created_micros(raw: &str) -> Option<i64> {
    let bytes = raw.as_bytes();
    // YYYY-MM-DDTHH:MM:SS is the minimum prefix.
    if bytes.len() < 19 || bytes[4] != b'-' || bytes[10] != b'T' {
        return None;
    }
    let year: i64 = raw.get(0..4)?.parse().ok()?;
    let month: u32 = raw.get(5..7)?.parse().ok()?;
    let day: u32 = raw.get(8..10)?.parse().ok()?;
    let hour: i64 = raw.get(11..13)?.parse().ok()?;
    let minute: i64 = raw.get(14..16)?.parse().ok()?;
    let second: i64 = raw.get(17..19)?.parse().ok()?;

    let days = days_from_civil(year, month, day)?;
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second;
    let micros = fractional_micros(&raw[19..]);
    Some(secs * 1_000_000 + micros)
}

/// Days since the Unix epoch for a civil date, by Howard Hinnant's algorithm.
fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(month);
    let d = i64::from(day);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Microseconds from the fractional-seconds tail of an RFC3339 string.
///
/// Accepts `.ffffff` of up to six digits before the timezone marker; any tail
/// that is not a fractional part contributes zero (seconds resolution is still
/// correct).
fn fractional_micros(tail: &str) -> i64 {
    let Some(rest) = tail.strip_prefix('.') else {
        return 0;
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).take(6).collect();
    if digits.is_empty() {
        return 0;
    }
    let scale = 10_i64.pow(6 - digits.len() as u32);
    digits.parse::<i64>().unwrap_or(0) * scale
}

#[cfg(test)]
mod tests {
    use super::parse_created_micros;

    #[test]
    fn epoch_start_parses_to_zero() {
        assert_eq!(parse_created_micros("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn a_known_instant_parses_to_its_epoch_micros() {
        // 2021-01-01T00:00:00Z == 1609459200 seconds since the epoch.
        assert_eq!(
            parse_created_micros("2021-01-01T00:00:00Z"),
            Some(1_609_459_200_000_000)
        );
    }

    #[test]
    fn fractional_seconds_contribute_micros() {
        assert_eq!(parse_created_micros("1970-01-01T00:00:00.5Z"), Some(500_000));
        assert_eq!(
            parse_created_micros("1970-01-01T00:00:00.000123Z"),
            Some(123)
        );
    }

    #[test]
    fn a_malformed_timestamp_is_none() {
        assert_eq!(parse_created_micros("not-a-date"), None);
        assert_eq!(parse_created_micros("2021/01/01"), None);
    }
}
