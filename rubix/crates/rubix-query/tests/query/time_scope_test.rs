//! Integration: the structured time scope injects a UTC window and an
//! epoch-aligned bucket, and the rewritten SQL runs in DataFusion.
//!
//! Proves the §5 fix (`rubix/docs/design/DASHBOARDS-SCOPE.md`): a board sends an
//! absolute UTC window + grain, the backend rewrites `$__timeFilter`/
//! `$__timeBucket` into real SQL against the canonical `created` column, and the
//! result is the rows inside the window, bucketed on epoch-aligned boundaries —
//! regardless of any client timezone.

#[path = "../fixture/mod.rs"]
mod fixture;

use datafusion::arrow::array::{Array, Int64Array, TimestampMicrosecondArray};
use rubix_core::{Record, Role, create_record};
use rubix_query::{Grain, TimeBound, TimeScope, apply_time_scope, ensure_read_only, run};

use fixture::open::{open_query_store, scoped_session_for};

/// Build a record whose stored `created` instant we control, by writing it and
/// then reading it back — the scan reads `created` as the store assigns it, so we
/// assert on relative windowing rather than an exact wall clock. To get
/// deterministic instants we instead seed via explicit content and filter on the
/// bucket count math, which is independent of the absolute `created` values.
#[tokio::test]
async fn a_window_filter_and_bucket_run_in_datafusion() {
    let database = "query_time_scope";
    let handle = open_query_store(database).await;
    for record in [
        Record::new("rubix", serde_json::json!({ "temp": 1 })),
        Record::new("rubix", serde_json::json!({ "temp": 2 })),
        Record::new("rubix", serde_json::json!({ "temp": 3 })),
    ] {
        create_record(handle.raw(), &record).await.expect("seed");
    }

    let (_principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Viewer).await;

    // A wide window that certainly contains every just-written record, bucketed by
    // hour. `now` is taken far in the future so the relative-token path is
    // exercised end to end.
    let now_ms = 4_102_444_800_000; // 2100-01-01T00:00:00Z, comfortably after seeds.
    let scope = TimeScope {
        from: TimeBound::Absolute(0),
        to: TimeBound::Relative("now".to_owned()),
        grain: Some(Grain::Hour),
        target_points: None,
    };

    let chart_sql = "SELECT $__timeBucket(created) AS bucket, count(*) AS n \
                     FROM record WHERE $__timeFilter(created) \
                     GROUP BY bucket ORDER BY bucket";
    let sql = apply_time_scope(chart_sql, &scope, now_ms).expect("expand macros");
    // The rewritten statement must still pass the read-only guard.
    ensure_read_only(&sql).expect("rewritten SQL is a single read-only statement");

    let batches = run(session.connection(), &sql).await.expect("run windowed query");

    let total: i64 = batches
        .iter()
        .flat_map(|b| {
            let n = b
                .column_by_name("n")
                .expect("n column")
                .as_any()
                .downcast_ref::<Int64Array>()
                .expect("int64 n");
            (0..n.len()).map(|i| n.value(i)).collect::<Vec<_>>()
        })
        .sum();
    assert_eq!(total, 3, "every seeded record falls inside the window");

    // Each bucket boundary is an epoch-aligned multiple of the hour grain.
    let width = Grain::Hour.width_micros();
    for batch in &batches {
        let buckets = batch
            .column_by_name("bucket")
            .expect("bucket column")
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .expect("timestamp bucket");
        for i in 0..buckets.len() {
            assert_eq!(buckets.value(i) % width, 0, "bucket is epoch-aligned");
        }
    }
}

#[tokio::test]
async fn a_narrow_past_window_excludes_freshly_written_rows() {
    let database = "query_time_scope_narrow";
    let handle = open_query_store(database).await;
    for record in [Record::new("rubix", serde_json::json!({ "temp": 1 }))] {
        create_record(handle.raw(), &record).await.expect("seed");
    }

    let (_principal, session) =
        scoped_session_for(&handle, database, "bob", "rubix", Role::Viewer).await;

    // A window entirely in 1970 cannot contain a record written now.
    let scope = TimeScope {
        from: TimeBound::Absolute(0),
        to: TimeBound::Absolute(1_000),
        grain: None,
        target_points: None,
    };
    let sql = apply_time_scope(
        "SELECT count(*) AS n FROM record WHERE $__timeFilter(created)",
        &scope,
        1_000,
    )
    .expect("expand");

    let batches = run(session.connection(), &sql).await.expect("run");
    let n = batches[0]
        .column_by_name("n")
        .expect("n")
        .as_any()
        .downcast_ref::<Int64Array>()
        .expect("int64")
        .value(0);
    assert_eq!(n, 0, "a 1970 window excludes a row written now");
}
