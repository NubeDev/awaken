//! Parquet partition addressing for the `his` cold tier.
//!
//! A partition groups samples by point and UTC day, so a point's history lands
//! in dated files under a per-point prefix:
//!
//! ```text
//! point=<point_id>/date=<YYYY-MM-DD>/<bucket>.parquet
//! ```
//!
//! The point/date prefixes keep listing cheap for point- and time-scoped reads;
//! the leaf file name is the flush bucket's start instant, so repeated flushes
//! of the same day append distinct files rather than overwriting.

use chrono::{DateTime, NaiveDate, Utc};
use datafusion::object_store::path::Path as StorePath;

/// The day a sample's timestamp falls in, used as its partition key.
pub(crate) fn day_of(ts: &DateTime<Utc>) -> NaiveDate {
    ts.date_naive()
}

/// The object-store prefix holding every partition file (all points, all days).
pub(crate) fn root_prefix() -> StorePath {
    StorePath::from("his")
}

/// The object-store path for a partition file: the `point`/`date` prefix plus a
/// leaf named for the flush instant (microsecond precision, so concurrent
/// flushes of one day never collide).
pub(crate) fn partition_path(point_id: &str, day: NaiveDate, flush: &DateTime<Utc>) -> StorePath {
    let leaf = flush.format("%Y%m%dT%H%M%S%6f").to_string();
    StorePath::from(format!(
        "his/point={point_id}/date={day}/{leaf}.parquet"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_is_utc_calendar_date() {
        let ts = DateTime::parse_from_rfc3339("2026-01-02T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(day_of(&ts), NaiveDate::from_ymd_opt(2026, 1, 2).unwrap());
    }

    #[test]
    fn partition_path_nests_point_and_date() {
        let day = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let flush = DateTime::parse_from_rfc3339("2026-01-03T04:05:06.123456Z")
            .unwrap()
            .with_timezone(&Utc);
        let path = partition_path("p1", day, &flush);
        assert_eq!(
            path.as_ref(),
            "his/point=p1/date=2026-01-02/20260103T040506123456.parquet"
        );
    }
}
