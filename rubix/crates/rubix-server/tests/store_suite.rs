//! Shared store test suite. Every assertion runs against the SQLite backend
//! unconditionally; when `RUBIX_TEST_PG` holds a `postgres://` url the same
//! suite runs against that Postgres backend too, so the cloud store is held to
//! the identical contract (WS-05). Absent the env var the Postgres pass skips
//! cleanly — it is not `#[ignore]`d, so CI without a database still exercises
//! SQLite and the harness end to end.

use chrono::Utc;
use rubix_core::Equip;
use rubix_core::{
    GridLayout, HisSample, Point, PointKind, PointValue, PriorityArray, Site, Spark, SparkSeverity,
    TagSet, Widget, WidgetKind, WidgetSettings,
};
use rubix_flow::BoardGraph;
use rubix_server::agent::{PendingWrite, RunOrigin, RunRecord, RunStatus};
use rubix_server::auth::{pat, Role, Scope, TokenRecord};
use rubix_server::scheduler::{BoardRecord, Trigger};
use rubix_server::store::Store;
use uuid::Uuid;

/// An empty board graph; the store persists it as opaque JSON, so node content
/// is irrelevant to the persistence contract.
fn empty_graph() -> BoardGraph {
    BoardGraph {
        nodes: Vec::new(),
        connections: Vec::new(),
    }
}

/// Open a fresh SQLite store in a temp dir.
fn sqlite_store() -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("suite.db")).expect("open sqlite");
    (store, dir)
}

/// Open the Postgres store named by `RUBIX_TEST_PG`, wiping every table first
/// so each run starts clean. `None` when the env var is unset.
#[cfg(feature = "cloud")]
fn postgres_store() -> Option<Store> {
    let url = std::env::var("RUBIX_TEST_PG").ok()?;
    let store = Store::connect(&url).expect("connect postgres");
    store.truncate_all_for_tests().expect("truncate");
    Some(store)
}

fn site() -> Site {
    Site {
        id: Uuid::new_v4(),
        org: "nube".into(),
        slug: "hq".into(),
        display_name: "HQ".into(),
        tags: TagSet::default(),
        created_at: Utc::now(),
    }
}

fn writable_point(equip_id: Uuid, slug: &str) -> Point {
    Point {
        id: Uuid::new_v4(),
        equip_id,
        slug: slug.into(),
        display_name: slug.into(),
        kind: PointKind::Cmd,
        unit: Some("degC".into()),
        tags: TagSet::default(),
        priority_array: PriorityArray::new(),
        cur_value: None,
        cur_ts: None,
        created_at: Utc::now(),
    }
}

/// The full contract every backend must satisfy. Run against a store that is
/// known to be empty.
fn run_suite(store: &Store) {
    // Sites: create, get, list, filter by org.
    let s = site();
    store.create_site(&s).unwrap();
    assert_eq!(store.get_site(s.id).unwrap().slug, "hq");
    assert_eq!(store.list_sites(None).unwrap().len(), 1);
    assert_eq!(store.list_sites(Some("nube")).unwrap().len(), 1);
    assert_eq!(store.list_sites(Some("other")).unwrap().len(), 0);

    // A duplicate (org, slug) is a conflict, not a silent overwrite.
    let mut dup = site();
    dup.id = Uuid::new_v4();
    assert!(store.create_site(&dup).is_err());

    // Equips: create under the site, foreign-key enforced.
    let equip = Equip {
        id: Uuid::new_v4(),
        site_id: s.id,
        path: "ahu-3".into(),
        display_name: "AHU 3".into(),
        tags: TagSet::default(),
        created_at: Utc::now(),
    };
    store.create_equip(&equip).unwrap();
    assert_eq!(store.list_equips(Some(s.id), &[]).unwrap().len(), 1);
    // An equip under a missing site fails closed.
    let orphan = Equip {
        id: Uuid::new_v4(),
        site_id: Uuid::new_v4(),
        path: "x".into(),
        display_name: "x".into(),
        tags: TagSet::default(),
        created_at: Utc::now(),
    };
    assert!(store.create_equip(&orphan).is_err());

    // Points: create, keyexpr resolution both directions.
    let point = writable_point(equip.id, "sp");
    store.create_point(&point).unwrap();
    let keyexpr = store.point_keyexpr(point.id).unwrap();
    assert_eq!(keyexpr, "nube/hq/ahu-3/sp");
    assert_eq!(store.point_by_keyexpr(&keyexpr).unwrap(), point.id);
    assert_eq!(store.site_id_by_prefix("nube/hq").unwrap(), s.id);
    assert_eq!(store.owned_site_prefixes().unwrap(), vec!["nube/hq"]);
    assert_eq!(store.all_point_keys().unwrap().len(), 1);

    // Command path: a priority write sets the effective value and logs history.
    let now = Utc::now();
    let commanded = store
        .command_point(point.id, 8, Some(PointValue::Number(21.5)), now)
        .unwrap();
    assert_eq!(commanded.cur_value, Some(PointValue::Number(21.5)));
    let his = store.his_query(point.id, None, None, 10).unwrap();
    assert_eq!(his.len(), 1);
    assert_eq!(his[0].value, PointValue::Number(21.5));

    // Relinquish clears the slot; with no other slot the value drops to none.
    let relinquished = store.command_point(point.id, 8, None, Utc::now()).unwrap();
    assert_eq!(relinquished.cur_value, None);

    // Sensor ingest + batch history insert.
    let sensor = Point {
        kind: PointKind::Sensor,
        ..writable_point(equip.id, "temp")
    };
    store.create_point(&sensor).unwrap();
    store
        .ingest_cur(
            sensor.id,
            &HisSample {
                ts: Utc::now(),
                value: PointValue::Number(19.0),
            },
        )
        .unwrap();
    let inserted = store
        .his_insert(
            sensor.id,
            &[
                HisSample {
                    ts: Utc::now(),
                    value: PointValue::Number(18.0),
                },
                HisSample {
                    ts: Utc::now(),
                    value: PointValue::Number(17.5),
                },
            ],
        )
        .unwrap();
    assert_eq!(inserted, 2);

    // Sparks: create, list, filter, ack.
    let spark = Spark {
        id: Uuid::new_v4(),
        site_id: s.id,
        rule: "simultaneous-heat-cool".into(),
        severity: SparkSeverity::Fault,
        message: "AHU-3 heating and cooling at once".into(),
        point_ids: vec![point.id],
        ts: Utc::now(),
        acknowledged: false,
    };
    store.create_spark(&spark).unwrap();
    assert_eq!(store.list_sparks(Some(s.id), None, None).unwrap().len(), 1);
    assert_eq!(
        store
            .list_sparks(None, Some("simultaneous-heat-cool"), Some(false))
            .unwrap()
            .len(),
        1
    );
    store.ack_spark(spark.id).unwrap();
    assert_eq!(store.list_sparks(None, None, Some(true)).unwrap().len(), 1);

    // Dashboards + widgets: a tile pins onto a dashboard.
    let dashboard_id = store.default_dashboard_for_site(s.id).unwrap();
    assert_eq!(store.list_dashboards(&s.org, Some(s.id)).unwrap().len(), 1);
    let widget = Widget {
        id: Uuid::new_v4(),
        dashboard_id,
        site_id: s.id,
        kind: WidgetKind::PointValue,
        title: "AHU-3 SP".into(),
        target: keyexpr.clone(),
        query: None,
        settings: None,
        created_at: Utc::now(),
    };
    store.create_widget(&widget).unwrap();
    assert_eq!(store.list_widgets(Some(s.id), None).unwrap().len(), 1);
    assert_eq!(
        store.list_widgets(None, Some(dashboard_id)).unwrap().len(),
        1
    );

    // Settings round-trip: a tile pins with no layout, then the builder sets a
    // grid cell + chart config; clearing returns it to the default rendering.
    assert!(store.get_widget(widget.id).unwrap().settings.is_none());
    let settings = WidgetSettings {
        layout: Some(GridLayout {
            x: 2,
            y: 0,
            w: 4,
            h: 3,
        }),
        config: Some(serde_json::json!({ "type": "bar" })),
    };
    let updated = store
        .update_widget_settings(widget.id, Some(&settings))
        .unwrap();
    assert_eq!(updated.settings.as_ref(), Some(&settings));
    assert_eq!(
        store.get_widget(widget.id).unwrap().settings,
        Some(settings)
    );
    let cleared = store.update_widget_settings(widget.id, None).unwrap();
    assert!(cleared.settings.is_none());

    // Boards: versioning, latest-per-scope, get, delete. An org-level flow and a
    // site-scoped flow can share a slug; they are distinct boards.
    let board = BoardRecord {
        id: Uuid::new_v4(),
        org: s.org.clone(),
        site_id: None,
        slug: "reset".into(),
        version: store.next_board_version(&s.org, None, "reset").unwrap(),
        display_name: "Reset".into(),
        enabled: true,
        trigger: Trigger::Interval { seconds: 60 },
        graph: empty_graph(),
        created_at: Utc::now(),
    };
    assert_eq!(board.version, 1);
    store.create_board(&board).unwrap();
    let v2 = BoardRecord {
        id: Uuid::new_v4(),
        version: store.next_board_version(&s.org, None, "reset").unwrap(),
        ..board.clone()
    };
    assert_eq!(v2.version, 2);
    store.create_board(&v2).unwrap();
    // Re-fetch by id returns the exact version it names.
    assert_eq!(store.get_board_by_id(board.id).unwrap().version, 1);
    assert_eq!(store.get_board(&s.org, None, "reset").unwrap().version, 2);

    // A site-scoped flow with the SAME slug is a separate board (scope wins).
    let site_board = BoardRecord {
        id: Uuid::new_v4(),
        org: s.org.clone(),
        site_id: Some(s.id),
        slug: "reset".into(),
        version: store.next_board_version(&s.org, Some(s.id), "reset").unwrap(),
        ..board.clone()
    };
    assert_eq!(site_board.version, 1, "site scope versions independently");
    store.create_board(&site_board).unwrap();
    // The org-list (site_id None) sees both; the site filter sees only the site one.
    assert_eq!(store.latest_boards(&s.org, None).unwrap().len(), 2);
    assert_eq!(store.latest_boards(&s.org, Some(s.id)).unwrap().len(), 1);

    store.delete_board(&s.org, None, "reset").unwrap();
    assert!(store.get_board(&s.org, None, "reset").is_err());
    // The site-scoped flow of the same slug is untouched by the org-level delete.
    assert!(store.get_board(&s.org, Some(s.id), "reset").is_ok());

    // Runs: persist a suspended run with a held write, then settle it once.
    let run = RunRecord {
        id: format!("run-{}", Uuid::new_v4()),
        thread_id: "thread-1".into(),
        origin: RunOrigin::Chat,
        status: RunStatus::Suspended,
        response: "awaiting approval".into(),
        steps: 3,
        pending_write: Some(PendingWrite {
            point: keyexpr.clone(),
            priority: 12,
            value: PointValue::Number(22.0),
            agent_min_priority: 13,
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.create_run(&run).unwrap();
    assert_eq!(
        store.list_runs(Some(RunStatus::Suspended)).unwrap().len(),
        1
    );
    let held = store.get_run(&run.id).unwrap();
    assert_eq!(held.pending_write.as_ref().unwrap().priority, 12);
    let settled = store
        .settle_suspended_run(&run.id, RunStatus::Resumed)
        .unwrap();
    assert_eq!(settled.id, run.id);
    // A second settle is a conflict (one-shot resume).
    assert!(store
        .settle_suspended_run(&run.id, RunStatus::Resumed)
        .is_err());
    assert_eq!(store.get_run(&run.id).unwrap().status, RunStatus::Resumed);

    // Tokens: issue, look up by secret hash (the verifier's path), revoke.
    let minted = pat::mint();
    let token = TokenRecord {
        id: minted.id.clone(),
        secret_hash: minted.secret_hash.clone(),
        name: "driver-1".into(),
        role: Role::Service,
        scope: Scope::org("nube"),
        created_at: Utc::now(),
        revoked_at: None,
    };
    store.create_token(&token).unwrap();
    assert_eq!(store.list_tokens().unwrap().len(), 1);
    let found = store
        .token_by_hash(&minted.secret_hash)
        .unwrap()
        .expect("token by hash");
    assert!(found.is_active());
    assert_eq!(found.role, Role::Service);
    assert_eq!(found.scope, Scope::org("nube"));
    store.revoke_token(&minted.id).unwrap();
    let revoked = store
        .token_by_hash(&minted.secret_hash)
        .unwrap()
        .expect("token still present after revoke");
    assert!(!revoked.is_active());
    // Revoking an absent token is a not-found error.
    assert!(store.revoke_token("nonexistent").is_err());

    // Cascade: deleting the site removes its dependents.
    store.delete_site(s.id).unwrap();
    assert!(store.get_site(s.id).is_err());
    assert_eq!(store.list_equips(None, &[]).unwrap().len(), 0);
    assert_eq!(store.list_widgets(None, None).unwrap().len(), 0);
}

#[test]
fn sqlite_satisfies_the_store_contract() {
    let (store, _dir) = sqlite_store();
    run_suite(&store);
}

#[cfg(feature = "cloud")]
#[test]
fn postgres_satisfies_the_store_contract() {
    let Some(store) = postgres_store() else {
        eprintln!("RUBIX_TEST_PG unset; skipping the Postgres store contract pass");
        return;
    };
    run_suite(&store);
}
