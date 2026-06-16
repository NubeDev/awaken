//! Integration: the scoped-context cache keys on the principal, never the SQL.
//!
//! The §4a security contract (`rubix/docs/design/DASHBOARDS-SCOPE.md`): the cache
//! holds the per-principal **scanned context**, so two principals running the
//! identical statement must get their own row-scoped rows — a results-by-SQL cache
//! would leak one principal's rows to the other. These tests prove (i) no
//! cross-principal hit, and (ii) a repeated tick for one principal reuses the
//! cached scan rather than rebuilding it.

#[path = "../fixture/mod.rs"]
mod fixture;

use std::time::Duration;

use datafusion::arrow::array::Int64Array;
use rubix_core::{Record, Role, create_record};
use rubix_query::{ContextCache, ScopeIdentity, build_context_cached};

use fixture::open::{open_query_store, scoped_session_for};

/// Run a scalar `count(*)` over `record` on a cached context for `scope`.
async fn count_records(
    session: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    cache: &ContextCache,
    scope: &ScopeIdentity,
) -> i64 {
    let ctx = build_context_cached(session, cache, scope)
        .await
        .expect("cached context");
    let batches = ctx
        .sql("SELECT count(*) AS n FROM record")
        .await
        .expect("plan")
        .collect()
        .await
        .expect("collect");
    batches[0]
        .column_by_name("n")
        .expect("n")
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("int64")
        .value(0)
}

#[tokio::test]
async fn two_principals_running_identical_sql_get_their_own_rows() {
    let database = "query_cache_scope";
    let handle = open_query_store(database).await;

    // Namespace A has two records; namespace B has three. Each principal is scoped
    // to its own namespace by SurrealDB row-level permissions.
    for _ in 0..2 {
        create_record(handle.raw(), &Record::new("ns-a", serde_json::json!({ "k": 1 })))
            .await
            .expect("seed a");
    }
    for _ in 0..3 {
        create_record(handle.raw(), &Record::new("ns-b", serde_json::json!({ "k": 1 })))
            .await
            .expect("seed b");
    }

    let (_pa, sa) = scoped_session_for(&handle, database, "alice", "ns-a", Role::Viewer).await;
    let (_pb, sb) = scoped_session_for(&handle, database, "bob", "ns-b", Role::Viewer).await;

    let cache = ContextCache::default();
    let scope_a = ScopeIdentity::new("ns-a", "alice");
    let scope_b = ScopeIdentity::new("ns-b", "bob");

    // Alice queries first and seeds her scan into the cache.
    let a = count_records(sa.connection(), &cache, &scope_a).await;
    assert_eq!(a, 2, "alice sees only namespace A's rows");

    // Bob runs the IDENTICAL SQL. A results-by-SQL cache would serve alice's 2
    // rows; the per-principal key means bob scans his own namespace.
    let b = count_records(sb.connection(), &cache, &scope_b).await;
    assert_eq!(b, 3, "bob sees only namespace B's rows — no cross-principal hit");

    // Two distinct principals → two distinct cache entries.
    assert_eq!(cache.len(), 2, "each principal has its own cached scan");
}

#[tokio::test]
async fn a_repeated_tick_reuses_the_cached_scan() {
    let database = "query_cache_reuse";
    let handle = open_query_store(database).await;
    create_record(handle.raw(), &Record::new("rubix", serde_json::json!({ "k": 1 })))
        .await
        .expect("seed");

    let (_p, session) =
        scoped_session_for(&handle, database, "carol", "rubix", Role::Viewer).await;

    let cache = ContextCache::new(10, Duration::from_secs(60));
    let scope = ScopeIdentity::new("rubix", "carol");

    // First tick scans and caches.
    let first = count_records(session.connection(), &cache, &scope).await;
    assert_eq!(first, 1);
    assert_eq!(cache.len(), 1, "the scan was cached");

    // A write happens but the cache is NOT invalidated here (the transport layer
    // owns invalidation). A second tick within the TTL must therefore reuse the
    // cached scan and still report the pre-write count — proving it did not
    // rebuild the context.
    create_record(handle.raw(), &Record::new("rubix", serde_json::json!({ "k": 2 })))
        .await
        .expect("seed 2");
    let second = count_records(session.connection(), &cache, &scope).await;
    assert_eq!(second, 1, "the tick reused the cached scan, not a fresh rescan");

    // After invalidation, the next tick rescans and sees the new row.
    cache.invalidate_namespace("rubix");
    let third = count_records(session.connection(), &cache, &scope).await;
    assert_eq!(third, 2, "post-invalidation the scan reflects the write");
}
