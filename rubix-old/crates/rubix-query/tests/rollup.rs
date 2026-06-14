//! Time-bucketed `his` rollups via the query engine.

use rubix_query::{Aggregate, Interval, QueryEngine, RollupSpec};
use rusqlite::Connection;
use tempfile::TempDir;

/// Full canonical schema (rollup registers all tables, so every one must exist).
const SCHEMA: &str = "
CREATE TABLE sites (id TEXT PRIMARY KEY, org TEXT, slug TEXT, display_name TEXT, tags TEXT, created_at TEXT);
CREATE TABLE equips (id TEXT PRIMARY KEY, site_id TEXT, path TEXT, display_name TEXT, tags TEXT, created_at TEXT);
CREATE TABLE points (id TEXT PRIMARY KEY, equip_id TEXT, slug TEXT, display_name TEXT, kind TEXT, unit TEXT, tags TEXT, priority_array TEXT, cur_value TEXT, cur_ts TEXT, created_at TEXT);
CREATE TABLE his (point_id TEXT NOT NULL, ts TEXT NOT NULL, value TEXT NOT NULL);
CREATE TABLE sparks (id TEXT PRIMARY KEY, site_id TEXT, rule TEXT, severity TEXT, message TEXT, point_ids TEXT, ts TEXT, acknowledged INTEGER);
";

fn engine_with_his(samples: &[(&str, &str, &str)]) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rubix.db");
    let conn = Connection::open(&path).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    for (pid, ts, value) in samples {
        conn.execute(
            "INSERT INTO his (point_id, ts, value) VALUES (?1, ?2, ?3)",
            rusqlite::params![pid, ts, value],
        )
        .expect("insert his");
    }
    (dir, path)
}

#[tokio::test]
async fn hourly_average_buckets_one_point() {
    let (_dir, path) = engine_with_his(&[
        ("p1", "2026-01-01T00:05:00Z", "20.0"),
        ("p1", "2026-01-01T00:35:00Z", "22.0"),
        ("p1", "2026-01-01T01:15:00Z", "30.0"),
    ]);
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .his_rollup(&RollupSpec {
            points: vec!["p1".into()],
            interval: Interval::Hour,
            agg: Aggregate::Avg,
            start: None,
            end: None,
        })
        .await
        .expect("rollup");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["value"], 21.0);
    assert_eq!(rows[0]["samples"], 2);
    assert_eq!(rows[1]["value"], 30.0);
    assert_eq!(rows[1]["samples"], 1);
}

#[tokio::test]
async fn max_with_time_bounds() {
    let (_dir, path) = engine_with_his(&[
        ("p1", "2026-01-01T00:10:00Z", "5.0"),
        ("p1", "2026-01-01T00:50:00Z", "9.0"),
        ("p1", "2026-01-01T02:00:00Z", "99.0"), // outside the upper bound
    ]);
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .his_rollup(&RollupSpec {
            points: vec!["p1".into()],
            interval: Interval::Hour,
            agg: Aggregate::Max,
            start: Some("2026-01-01T00:00:00Z".into()),
            end: Some("2026-01-01T01:00:00Z".into()),
        })
        .await
        .expect("rollup");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["value"], 9.0);
}

#[tokio::test]
async fn groups_per_point() {
    let (_dir, path) = engine_with_his(&[
        ("p1", "2026-01-01T00:05:00Z", "1.0"),
        ("p2", "2026-01-01T00:05:00Z", "2.0"),
    ]);
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .his_rollup(&RollupSpec {
            points: vec!["p1".into(), "p2".into()],
            interval: Interval::Hour,
            agg: Aggregate::Sum,
            start: None,
            end: None,
        })
        .await
        .expect("rollup");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["point_id"], "p1");
    assert_eq!(rows[1]["point_id"], "p2");
}

#[tokio::test]
async fn empty_points_is_empty() {
    let (_dir, path) = engine_with_his(&[]);
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .his_rollup(&RollupSpec {
            points: vec![],
            interval: Interval::Hour,
            agg: Aggregate::Avg,
            start: None,
            end: None,
        })
        .await
        .expect("rollup");

    assert!(rows.is_empty());
}

#[tokio::test]
async fn rejects_quote_in_point_id() {
    let (_dir, path) = engine_with_his(&[]);
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let err = engine
        .his_rollup(&RollupSpec {
            points: vec!["p1' OR '1'='1".into()],
            interval: Interval::Hour,
            agg: Aggregate::Avg,
            start: None,
            end: None,
        })
        .await;

    assert!(err.is_err(), "injection attempt must be rejected");
}
