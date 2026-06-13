//! End-to-end: variable tokens lower into bound DataFusion parameters and the
//! result is correct, including the injection case (a payload binds as a literal
//! and selects nothing rather than executing).

use rubix_query::{QueryEngine, QueryVariable, Scalar, VarValue};
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

fn seed_db() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rubix.db");
    let conn = Connection::open(&path).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    for (id, org, slug) in [("s1", "acme", "hq"), ("s2", "acme", "lab"), ("s3", "beta", "site")] {
        conn.execute(
            "INSERT INTO sites (id, org, slug, display_name, tags, created_at) \
             VALUES (?1, ?2, ?3, ?3, '{}', '2026-01-01T00:00:00Z')",
            rusqlite::params![id, org, slug],
        )
        .expect("site");
    }
    (dir, path)
}

fn one(name: &str, value: &str) -> QueryVariable {
    QueryVariable {
        name: name.to_string(),
        value: VarValue::One(Scalar::Text(value.to_string())),
    }
}

#[tokio::test]
async fn single_variable_binds_and_filters() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .query_with_variables(
            "SELECT slug FROM sites WHERE org = $org ORDER BY slug",
            &[one("org", "acme")],
        )
        .await
        .expect("query");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["slug"], "hq");
    assert_eq!(rows[1]["slug"], "lab");
}

#[tokio::test]
async fn sql_in_expands_multi_value() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let var = QueryVariable {
        name: "ids".into(),
        value: VarValue::Many(vec![Scalar::Text("s1".into()), Scalar::Text("s3".into())]),
    };
    let rows = engine
        .query_with_variables(
            "SELECT slug FROM sites WHERE id $__sqlIn(ids) ORDER BY slug",
            &[var],
        )
        .await
        .expect("query");

    // s1 (hq) and s3 (site) match; s2 (lab) does not.
    assert_eq!(rows.len(), 2);
}

#[tokio::test]
async fn injection_payload_binds_as_literal_and_selects_nothing() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    // The classic payload is bound as one literal string. No table is dropped;
    // it simply matches no `org`, so the result is empty.
    let rows = engine
        .query_with_variables(
            "SELECT slug FROM sites WHERE org = $org",
            &[one("org", "'); DROP TABLE sites; --")],
        )
        .await
        .expect("query");
    assert!(rows.is_empty());

    // The table still exists and still holds all three rows.
    let all = engine.query("SELECT id FROM sites").await.expect("query");
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn no_variables_matches_plain_query() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let rows = engine
        .query_with_variables("SELECT id FROM sites", &[])
        .await
        .expect("query");
    assert_eq!(rows.len(), 3);
}

#[tokio::test]
async fn unknown_variable_is_an_error() {
    let (_dir, path) = seed_db();
    let engine = QueryEngine::open(&path).await.expect("open engine");

    let err = engine
        .query_with_variables("SELECT id FROM sites WHERE org = $missing", &[])
        .await
        .unwrap_err();
    assert!(err.to_string().contains("missing"));
}
