//! A tenant-scoped session exposes the canonical tables as views filtered to
//! one `{org}/{site}`, so a scoped query can never read another tenant's rows.

use rubix_query::{QueryEngine, QueryScope};
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

/// Seed two tenants (`acme/hq` and `beta/lab`), each with one equip, one point,
/// one history sample, and one spark, so a scoped session must filter all five
/// canonical tables by `{org}/{site}`.
fn seed_two_tenants() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("rubix.db");
    let conn = Connection::open(&path).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    conn.execute_batch(
        "INSERT INTO sites (id, org, slug, display_name, tags, created_at) VALUES \
           ('s_a', 'acme', 'hq', 'HQ', '{}', '2026-01-01T00:00:00Z'), \
           ('s_b', 'beta', 'lab', 'Lab', '{}', '2026-01-01T00:00:00Z');
         INSERT INTO equips (id, site_id, path, display_name, tags, created_at) VALUES \
           ('e_a', 's_a', 'ahu-3', 'AHU-3', '{}', '2026-01-01T00:00:00Z'), \
           ('e_b', 's_b', 'ahu-9', 'AHU-9', '{}', '2026-01-01T00:00:00Z');
         INSERT INTO points (id, equip_id, slug, display_name, kind, unit, tags, \
           priority_array, cur_value, cur_ts, created_at) VALUES \
           ('p_a', 'e_a', 'temp', 'Temp', 'analog', 'C', '{}', '[]', '21.5', \
            '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'), \
           ('p_b', 'e_b', 'temp', 'Temp', 'analog', 'C', '{}', '[]', '9.9', \
            '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
         INSERT INTO his (point_id, ts, value) VALUES \
           ('p_a', '2026-01-01T00:00:00Z', '21.5'), \
           ('p_b', '2026-01-01T00:00:00Z', '9.9');
         INSERT INTO sparks (id, site_id, rule, severity, message, point_ids, ts) VALUES \
           ('sp_a', 's_a', 'heat-cool', 'warn', 'acme finding', '[]', '2026-01-01T00:00:00Z'), \
           ('sp_b', 's_b', 'heat-cool', 'warn', 'beta finding', '[]', '2026-01-01T00:00:00Z');",
    )
    .expect("seed");
    (dir, path)
}

#[tokio::test]
async fn scoped_session_sees_only_its_tenant_sites() {
    let (_dir, path) = seed_two_tenants();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let scope = QueryScope::new("acme", "hq").expect("scope");

    let rows = engine
        .scoped_query(&scope, "SELECT org, slug FROM sites")
        .await
        .expect("query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["org"], "acme");
    assert_eq!(rows[0]["slug"], "hq");
}

#[tokio::test]
async fn scoped_session_filters_points_his_and_sparks() {
    let (_dir, path) = seed_two_tenants();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let scope = QueryScope::new("acme", "hq").expect("scope");

    let points = engine
        .scoped_query(&scope, "SELECT cur_value FROM points")
        .await
        .expect("points");
    assert_eq!(points.len(), 1);
    assert_eq!(points[0]["cur_value"], "21.5");

    let his = engine
        .scoped_query(&scope, "SELECT value FROM his")
        .await
        .expect("his");
    assert_eq!(his.len(), 1);
    assert_eq!(his[0]["value"], "21.5");

    let sparks = engine
        .scoped_query(&scope, "SELECT message FROM sparks")
        .await
        .expect("sparks");
    assert_eq!(sparks.len(), 1);
    assert_eq!(sparks[0]["message"], "acme finding");
}

#[tokio::test]
async fn scoped_points_cur_keyexpr_stays_in_tenant() {
    let (_dir, path) = seed_two_tenants();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let scope = QueryScope::new("beta", "lab").expect("scope");

    let rows = engine
        .scoped_query(&scope, "SELECT keyexpr FROM points_cur")
        .await
        .expect("query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["keyexpr"], "beta/lab/ahu-9/temp");
}

#[tokio::test]
async fn scoped_session_cannot_name_a_sibling_via_explicit_predicate() {
    // Even a SELECT that explicitly asks for the other tenant returns nothing —
    // the canonical view is already filtered, so the predicate can only narrow.
    let (_dir, path) = seed_two_tenants();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let scope = QueryScope::new("acme", "hq").expect("scope");

    let rows = engine
        .scoped_query(&scope, "SELECT org FROM sites WHERE org = 'beta'")
        .await
        .expect("query");

    assert!(rows.is_empty());
}

#[tokio::test]
async fn scoped_session_exposes_no_unfiltered_base_table() {
    // The raw providers are embedded inline in each scoped view's plan, never
    // registered under a nameable table — so there is no second name (the bare
    // SQLite table) that would return every tenant's rows. Naming one fails to
    // plan rather than leaking the whole database.
    let (_dir, path) = seed_two_tenants();
    let engine = QueryEngine::open(&path).await.expect("open engine");
    let scope = QueryScope::new("acme", "hq").expect("scope");

    for candidate in [
        "SELECT * FROM __base_sites",
        "SELECT * FROM datafusion.public.sites",
    ] {
        // A name that resolves must still be the tenant-filtered view: at most
        // the one acme row, never both tenants. A name that does not resolve
        // (errors) is also fine — there is simply no unfiltered table to reach.
        if let Ok(rows) = engine.scoped_query(&scope, candidate).await {
            assert!(
                rows.len() <= 1,
                "`{candidate}` leaked {} rows past the tenant filter",
                rows.len()
            );
        }
    }
}
