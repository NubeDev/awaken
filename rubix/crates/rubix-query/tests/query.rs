//! End-to-end: build a SQLite database with the canonical schema, then query
//! it through the DataFusion surface.

use rubix_query::QueryEngine;
use rusqlite::Connection;
use tempfile::TempDir;

/// Minimal slice of the rubix store schema needed to exercise the surface.
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

fn seed_db() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rubix.db");
    let conn = Connection::open(&path).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    conn.execute(
        "INSERT INTO sites (id, org, slug, display_name, tags, created_at) \
         VALUES ('s1', 'acme', 'hq', 'HQ', '{}', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("site");
    conn.execute(
        "INSERT INTO points (id, equip_id, slug, display_name, kind, unit, tags, \
         priority_array, cur_value, cur_ts, created_at) VALUES \
         ('p1', 'e1', 'temp', 'Temp', 'analog', '°C', '{}', '[]', '21.5', \
         '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("point");
    (dir, path)
}

#[tokio::test]
async fn queries_registered_tables_by_bare_name() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .query("SELECT org, slug FROM sites WHERE org = 'acme'")
        .await
        .expect("query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["org"], "acme");
    assert_eq!(rows[0]["slug"], "hq");
}

#[tokio::test]
async fn joins_across_canonical_tables() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .query("SELECT slug, cur_value FROM points ORDER BY slug")
        .await
        .expect("query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["slug"], "temp");
    assert_eq!(rows[0]["cur_value"], "21.5");
}

#[tokio::test]
async fn empty_result_is_empty_vec() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine.query("SELECT * FROM sparks").await.expect("query");

    assert!(rows.is_empty());
}
