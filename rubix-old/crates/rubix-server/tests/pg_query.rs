//! Postgres federation for the DataFusion query surface (WS-05 follow-up).
//!
//! Under the cloud profile the canonical tables federate from Postgres through
//! the connector, so `/query` works against a Postgres store and a tenant-scoped
//! run's `query` tool stays confined to its `{org}/{site}`. The suite seeds two
//! tenants in a Postgres store and asserts both the unscoped surface (sees all)
//! and the scoped surface (sees only its tenant) read what the store wrote.
//!
//! Runs only when `RUBIX_TEST_PG` names a database; absent it, the test skips
//! cleanly (not `#[ignore]`d) so the cloud build without a database still
//! compiles and links this path.

#![cfg(feature = "cloud")]

use chrono::Utc;
use rubix_core::{Equip, Point, PointKind, PointValue, PriorityArray, Site, TagSet};
use rubix_query::{QueryEngine, QueryScope};
use rubix_server::store::Store;
use uuid::Uuid;

/// Truncate the Postgres store at `url` and seed both tenants, off the async
/// runtime. The store's synchronous Postgres client drives its own runtime and
/// panics if blocked on from inside the test's tokio runtime, so all store I/O
/// runs on a blocking thread; the query engine (async, its own connector) then
/// reads what was written. Returns the url when `RUBIX_TEST_PG` is set.
async fn seed_two_tenants() -> Option<String> {
    let url = std::env::var("RUBIX_TEST_PG").ok()?;
    let seed_url = url.clone();
    tokio::task::spawn_blocking(move || {
        let store = Store::connect(&seed_url).expect("connect postgres");
        store.truncate_all_for_tests().expect("truncate");
        seed_tenant(&store, "ten", "pa", 21.5);
        seed_tenant(&store, "ten", "pb", 9.9);
    })
    .await
    .expect("seed task");
    Some(url)
}

/// Seed one tenant `{org}/{slug}` with an `ahu-3/temp` point carrying `cur`.
fn seed_tenant(store: &Store, org: &str, slug: &str, cur: f64) {
    let site = Site {
        id: Uuid::new_v4(),
        org: org.into(),
        slug: slug.into(),
        display_name: slug.into(),
        tags: TagSet::default(),
        created_at: Utc::now(),
    };
    store.create_site(&site).expect("site");
    let equip = Equip {
        id: Uuid::new_v4(),
        site_id: site.id,
        path: "ahu-3".into(),
        display_name: "AHU 3".into(),
        tags: TagSet::default(),
        created_at: Utc::now(),
    };
    store.create_equip(&equip).expect("equip");
    let mut point = Point {
        id: Uuid::new_v4(),
        equip_id: equip.id,
        slug: "temp".into(),
        display_name: "Temp".into(),
        kind: PointKind::Cmd,
        unit: Some("degC".into()),
        tags: TagSet::default(),
        priority_array: PriorityArray::new(),
        cur_value: None,
        cur_ts: None,
        created_at: Utc::now(),
    };
    store.create_point(&point).expect("point");
    // Land a current value through the command path so `points_cur` has a row.
    point = store
        .command_point(point.id, 8, Some(PointValue::Number(cur)), Utc::now())
        .expect("command");
    assert_eq!(point.cur_value, Some(PointValue::Number(cur)));
}

#[tokio::test]
async fn postgres_federation_unscoped_sees_all_and_scoped_confines() {
    // One test (not two) so the shared `RUBIX_TEST_PG` database is seeded once
    // and not raced by parallel truncates.
    let Some(url) = seed_two_tenants().await else {
        eprintln!("RUBIX_TEST_PG unset; skipping the Postgres query federation pass");
        return;
    };

    let engine = QueryEngine::open_postgres(&url)
        .await
        .expect("open postgres query engine");

    // Unscoped surface federates every tenant from Postgres.
    let rows = engine
        .query("SELECT org, slug FROM sites ORDER BY slug")
        .await
        .expect("query");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["slug"], "pa");
    assert_eq!(rows[1]["slug"], "pb");

    // Scoped surface confines the federation to one tenant.
    let scope = QueryScope::new("ten", "pa").expect("scope");

    // Sites: only the scoped tenant.
    let rows = engine
        .scoped_query(&scope, "SELECT slug FROM sites")
        .await
        .expect("query");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["slug"], "pa");

    // Naming the sibling cannot escape the filter.
    let rows = engine
        .scoped_query(&scope, "SELECT slug FROM sites WHERE slug = 'pb'")
        .await
        .expect("query");
    assert!(rows.is_empty());

    // points_cur keyexprs stay inside the tenant.
    let rows = engine
        .scoped_query(&scope, "SELECT keyexpr, cur_value FROM points_cur")
        .await
        .expect("query");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["keyexpr"], "ten/pa/ahu-3/temp");
}
