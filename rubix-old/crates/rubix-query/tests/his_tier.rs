//! Two-tier `his`: write aged rows into the Parquet cold tier, then verify
//! `/query` and `/his/rollup` read across the SQLite hot tier and Parquet
//! partitions as one history.

use chrono::{DateTime, Utc};
use rubix_query::{write_partitions, Aggregate, HisRow, HisTier, Interval, QueryEngine, RollupSpec};
use rusqlite::Connection;
use tempfile::TempDir;

const SCHEMA: &str = "
CREATE TABLE sites (id TEXT PRIMARY KEY, org TEXT, slug TEXT, display_name TEXT, tags TEXT, created_at TEXT);
CREATE TABLE equips (id TEXT PRIMARY KEY, site_id TEXT, path TEXT, display_name TEXT, tags TEXT, created_at TEXT);
CREATE TABLE points (id TEXT PRIMARY KEY, equip_id TEXT, slug TEXT, display_name TEXT, kind TEXT, unit TEXT, tags TEXT, priority_array TEXT, cur_value TEXT, cur_ts TEXT, created_at TEXT);
CREATE TABLE his (point_id TEXT NOT NULL, ts TEXT NOT NULL, value TEXT NOT NULL);
CREATE TABLE sparks (id TEXT PRIMARY KEY, site_id TEXT, rule TEXT, severity TEXT, message TEXT, point_ids TEXT, ts TEXT, acknowledged INTEGER);
";

fn ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .expect("rfc3339")
        .with_timezone(&Utc)
}

/// A SQLite db seeded with the hot-tier rows, plus a cold-tier directory.
fn fixture(hot: &[(&str, &str, &str)]) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let db = dir.path().join("rubix.db");
    let cold = dir.path().join("his-parquet");
    let conn = Connection::open(&db).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    for (pid, t, v) in hot {
        conn.execute(
            "INSERT INTO his (point_id, ts, value) VALUES (?1, ?2, ?3)",
            rusqlite::params![pid, t, v],
        )
        .expect("insert hot");
    }
    (dir, db, cold)
}

#[tokio::test]
async fn query_unions_hot_and_cold_rows() {
    let (_dir, db, cold) = fixture(&[("p1", "2026-03-01T00:00:00.000000Z", "10.0")]);
    let tier = HisTier::open_local(&cold).expect("tier");

    // Flush an aged row into the Parquet cold tier.
    let store = tier.store();
    write_partitions(
        &store,
        &[HisRow {
            point_id: "p1".into(),
            ts: ts("2026-01-15T00:00:00Z"),
            value: "99.0".into(),
        }],
        ts("2026-03-02T00:00:00Z"),
    )
    .await
    .expect("flush");

    let engine = QueryEngine::open(&db)
        .await
        .expect("open")
        .with_his_tier(tier);

    let rows = engine
        .query("SELECT point_id, value FROM his ORDER BY ts")
        .await
        .expect("query");

    assert_eq!(rows.len(), 2, "both the cold and hot row are visible");
    assert_eq!(rows[0]["value"], "99.0"); // cold (Jan) sorts first
    assert_eq!(rows[1]["value"], "10.0"); // hot (Mar)
}

#[tokio::test]
async fn rollup_aggregates_across_the_tier_boundary() {
    // Two samples in the same hour: one already flushed to Parquet, one still
    // in SQLite. The average must span both tiers.
    let (_dir, db, cold) = fixture(&[("p1", "2026-01-01T00:45:00.000000Z", "30.0")]);
    let tier = HisTier::open_local(&cold).expect("tier");
    write_partitions(
        &tier.store(),
        &[HisRow {
            point_id: "p1".into(),
            ts: ts("2026-01-01T00:15:00Z"),
            value: "10.0".into(),
        }],
        ts("2026-02-01T00:00:00Z"),
    )
    .await
    .expect("flush");

    let engine = QueryEngine::open(&db)
        .await
        .expect("open")
        .with_his_tier(tier);

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

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["value"], 20.0, "avg(10,30) across both tiers");
    assert_eq!(rows[0]["samples"], 2);
}

#[tokio::test]
async fn without_a_tier_his_is_sqlite_only() {
    let (_dir, db, _cold) = fixture(&[("p1", "2026-01-01T00:00:00.000000Z", "5.0")]);
    let engine = QueryEngine::open(&db).await.expect("open");

    let rows = engine.query("SELECT value FROM his").await.expect("query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["value"], "5.0");
}

#[tokio::test]
async fn empty_cold_tier_reads_hot_only() {
    let (_dir, db, cold) = fixture(&[("p1", "2026-01-01T00:00:00.000000Z", "7.0")]);
    let tier = HisTier::open_local(&cold).expect("tier");
    let engine = QueryEngine::open(&db)
        .await
        .expect("open")
        .with_his_tier(tier);

    let rows = engine.query("SELECT value FROM his").await.expect("query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["value"], "7.0");
}
