//! End-to-end: time macros lower into bound DataFusion parameters and the
//! result is correct. Proves `$__timeFilter(ts)` returns only in-range rows, a
//! no-macro query is unaffected by a supplied range, and the resolved range
//! binds (an injected bound never reaches SQL).

use chrono::{TimeZone, Utc};
use rubix_query::{resolve_time_range, QueryEngine, TimeContext, TimeRangeSpec};
use rusqlite::Connection;
use tempfile::TempDir;

const SCHEMA: &str = "
CREATE TABLE sites (
    id TEXT PRIMARY KEY, org TEXT NOT NULL, slug TEXT NOT NULL,
    display_name TEXT NOT NULL, tags TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE equips (
    id TEXT PRIMARY KEY, site_id TEXT NOT NULL, path TEXT NOT NULL,
    display_name TEXT NOT NULL, tags TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE points (
    id TEXT PRIMARY KEY, equip_id TEXT NOT NULL, slug TEXT NOT NULL,
    display_name TEXT NOT NULL, kind TEXT NOT NULL, unit TEXT,
    tags TEXT NOT NULL, priority_array TEXT NOT NULL, cur_value TEXT,
    cur_ts TEXT, created_at TEXT NOT NULL
);
CREATE TABLE his (point_id TEXT NOT NULL, ts TEXT NOT NULL, value TEXT NOT NULL);
CREATE TABLE sparks (
    id TEXT PRIMARY KEY, site_id TEXT NOT NULL, rule TEXT NOT NULL,
    severity TEXT NOT NULL, message TEXT NOT NULL, point_ids TEXT NOT NULL,
    ts TEXT NOT NULL, acknowledged INTEGER NOT NULL DEFAULT 0
);
";

/// Seed `his` with one sample per hour across a day so a range filters a known
/// subset. Timestamps are RFC 3339 with offset, matching the `his.ts` Utf8 form.
fn seed_db() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rubix.db");
    let conn = Connection::open(&path).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    for hour in 0..24 {
        let ts = Utc
            .with_ymd_and_hms(2026, 6, 13, hour, 0, 0)
            .single()
            .unwrap()
            .to_rfc3339();
        conn.execute(
            "INSERT INTO his (point_id, ts, value) VALUES ('p1', ?1, '1')",
            rusqlite::params![ts],
        )
        .expect("his");
    }
    (dir, path)
}

/// A fixed range [06:00, 12:00) on the seeded day with a 1h bucket.
fn range_06_to_12() -> TimeContext {
    let spec = TimeRangeSpec {
        from: "2026-06-13T06:00:00Z".into(),
        to: "2026-06-13T12:00:00Z".into(),
        interval_secs: Some(3600),
    };
    resolve_time_range(&spec, Utc::now()).expect("resolve")
}

#[tokio::test]
async fn time_filter_returns_only_in_range_rows() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let time = range_06_to_12();

    let rows = engine
        .query_lowered(
            "SELECT ts FROM his WHERE $__timeFilter(ts) ORDER BY ts",
            &[],
            Some(&time),
        )
        .await
        .expect("query");

    // Hours 06,07,08,09,10,11 are in [06:00, 12:00); 12:00 is excluded.
    assert_eq!(rows.len(), 6);
}

#[tokio::test]
async fn from_and_to_macros_bind_the_resolved_bounds() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let time = range_06_to_12();

    let rows = engine
        .query_lowered(
            "SELECT ts FROM his WHERE ts >= $__from AND ts < $__to ORDER BY ts",
            &[],
            Some(&time),
        )
        .await
        .expect("query");
    assert_eq!(rows.len(), 6);
}

#[tokio::test]
async fn no_macro_query_is_unaffected_by_a_supplied_range() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let time = range_06_to_12();

    // A range is supplied but the SQL uses no time macro: all 24 rows return.
    let rows = engine
        .query_lowered("SELECT ts FROM his", &[], Some(&time))
        .await
        .expect("query");
    assert_eq!(rows.len(), 24);
}

#[tokio::test]
async fn time_macro_without_a_range_is_an_error() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let err = engine
        .query_lowered("SELECT ts FROM his WHERE $__timeFilter(ts)", &[], None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("time"));
}

#[tokio::test]
async fn injected_bound_binds_as_literal_and_never_executes() {
    // A payload in a range bound fails to resolve to an instant, so it is
    // refused before any SQL is built — it can never reach the engine.
    let spec = TimeRangeSpec {
        from: "'); DROP TABLE his; --".into(),
        to: "now".into(),
        interval_secs: None,
    };
    let err = resolve_time_range(&spec, Utc::now()).unwrap_err();
    assert!(err.to_string().contains("invalid time-range token"));
}
