//! Dev seed integration: the demo portfolio lands as real store rows with
//! populated priority arrays, history backfill, and sparks — and re-seeding is
//! idempotent.

use rubix_core::{PointKind, PointValue};
use rubix_server::seed::{seed_portfolio, spawn_dev_ticker};
use rubix_server::store::Store;

fn fresh_store() -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("seed.db")).expect("open store");
    (store, dir)
}

#[test]
fn seeds_the_full_portfolio() {
    let (store, _dir) = fresh_store();
    let report = seed_portfolio(&store).expect("seed");

    // 4 sites, 9 equips each, 20 points each, 7 sparks each.
    assert_eq!(report.sites, 4);
    assert_eq!(report.equips, 4 * 9);
    assert_eq!(report.points, 4 * 20);
    assert_eq!(report.sparks, 4 * 7);
    assert!(report.his_samples > 0);

    let sites = store.list_sites(Some("acme")).unwrap();
    assert_eq!(sites.len(), 4);
    let slugs: Vec<_> = sites.iter().map(|s| s.slug.as_str()).collect();
    assert!(slugs.contains(&"hq-tower"));
    assert!(slugs.contains(&"cold-store-3"));
}

#[test]
fn re_seeding_is_idempotent() {
    let (store, _dir) = fresh_store();
    let first = seed_portfolio(&store).expect("first seed");
    assert!(first.sites > 0);

    let second = seed_portfolio(&store).expect("re-seed");
    // Nothing new is created on a populated store.
    assert_eq!(second.sites, 0);
    assert_eq!(second.equips, 0);
    assert_eq!(second.points, 0);
    assert_eq!(second.sparks, 0);
    assert_eq!(second.his_samples, 0);

    // Counts unchanged after the second pass.
    assert_eq!(store.list_sites(Some("acme")).unwrap().len(), 4);
}

#[test]
fn ahu3_command_points_have_populated_priority_arrays() {
    let (store, _dir) = fresh_store();
    seed_portfolio(&store).expect("seed");

    let hq = store
        .list_sites(Some("acme"))
        .unwrap()
        .into_iter()
        .find(|s| s.slug == "hq-tower")
        .unwrap();
    let points = store.list_points(None, Some(hq.id), &[]).unwrap();

    let fan = points
        .iter()
        .find(|p| p.slug == "supply-fan-cmd" && p.kind == PointKind::Cmd)
        .expect("AHU command fan point");

    // Slots 8 (operator), 13 (agent ceiling), 16 (schedule) populated; the
    // lowest level number wins, so the effective value is the slot-8 command.
    assert_eq!(fan.priority_array.get(8).unwrap(), Some(&PointValue::Number(82.0)));
    assert_eq!(fan.priority_array.get(13).unwrap(), Some(&PointValue::Number(70.0)));
    assert_eq!(fan.priority_array.get(16).unwrap(), Some(&PointValue::Number(60.0)));
    assert_eq!(fan.cur_value, Some(PointValue::Number(82.0)));
}

#[test]
fn numeric_points_are_backfilled_with_history() {
    let (store, _dir) = fresh_store();
    seed_portfolio(&store).expect("seed");

    let hq = store
        .list_sites(Some("acme"))
        .unwrap()
        .into_iter()
        .find(|s| s.slug == "hq-tower")
        .unwrap();
    let points = store.list_points(None, Some(hq.id), &[]).unwrap();
    let kw = points.iter().find(|p| p.slug == "kw-total").unwrap();

    let his = store.his_query(kw.id, None, None, 10_000).unwrap();
    // 7 days × 48 samples/day.
    assert_eq!(his.len(), 7 * 48);
    // Samples are time-ordered ascending.
    assert!(his.windows(2).all(|w| w[0].ts <= w[1].ts));
}

#[test]
fn submeter_and_comfort_tags_are_present() {
    let (store, _dir) = fresh_store();
    seed_portfolio(&store).expect("seed");

    // The dashboard derives Load Breakdown from `submeter` and KPIs from
    // `meter`/`comfort`; those tag reads must resolve to seeded points.
    let submeters = store
        .list_points(None, None, &["submeter".to_string()])
        .unwrap();
    assert_eq!(submeters.len(), 4 * 5, "5 submeters per site");

    let comfort = store
        .list_points(None, None, &["comfort".to_string()])
        .unwrap();
    assert_eq!(comfort.len(), 4, "one comfort index per site");
}

#[test]
fn sparks_preserve_acknowledged_state() {
    let (store, _dir) = fresh_store();
    seed_portfolio(&store).expect("seed");

    let hq = store
        .list_sites(Some("acme"))
        .unwrap()
        .into_iter()
        .find(|s| s.slug == "hq-tower")
        .unwrap();
    let acked = store.list_sparks(Some(hq.id), None, Some(true)).unwrap();
    // sensor-drift and low-delta-t seed acknowledged.
    assert_eq!(acked.len(), 2);
    let open = store.list_sparks(Some(hq.id), None, Some(false)).unwrap();
    assert_eq!(open.len(), 5);
}

#[test]
fn seeds_one_stored_board_with_a_valid_graph() {
    let (store, _dir) = fresh_store();
    let report = seed_portfolio(&store).expect("seed");
    assert_eq!(report.boards, 1, "the demo board is seeded");

    let boards = store.latest_boards().unwrap();
    let board = boards
        .iter()
        .find(|b| b.slug == "ahu-3-discharge-reset")
        .expect("demo board listed");
    assert_eq!(board.display_name, "AHU-3 · Discharge Reset");
    assert!(board.enabled);
    // Manual: the scheduler never fires it; only /boards/{slug}/run does.
    assert!(!board.is_scheduled());

    // Every node names a registered rubix component, and the graph is wired.
    assert_eq!(board.graph.nodes.len(), 3);
    for node in &board.graph.nodes {
        assert!(
            rubix_flow::COMPONENTS.contains(&node.component.as_str()),
            "unknown component {}",
            node.component
        );
    }
    assert_eq!(board.graph.connections.len(), 2);
}

#[test]
fn re_seeding_does_not_duplicate_the_board() {
    let (store, _dir) = fresh_store();
    assert_eq!(seed_portfolio(&store).unwrap().boards, 1);
    assert_eq!(seed_portfolio(&store).unwrap().boards, 0);
    assert_eq!(store.latest_boards().unwrap().len(), 1);
}

#[tokio::test]
async fn dev_ticker_ingests_fresh_cur_for_seeded_sensors() {
    let (store, _dir) = fresh_store();
    seed_portfolio(&store).expect("seed");

    let hq = store
        .list_sites(Some("acme"))
        .unwrap()
        .into_iter()
        .find(|s| s.slug == "hq-tower")
        .unwrap();
    let temp = store
        .list_points(None, Some(hq.id), &[])
        .unwrap()
        .into_iter()
        .find(|p| p.slug == "discharge-temp" && p.kind == PointKind::Sensor)
        .unwrap();
    let before = store.his_query(temp.id, None, None, 100_000).unwrap().len();

    // No bus in tests; the ticker ingests through the store either way.
    let ticker = spawn_dev_ticker(store.clone(), None).expect("tickable sensors");
    // The interval fires immediately on the first tick.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    ticker.shutdown().await;

    let after = store.his_query(temp.id, None, None, 100_000).unwrap().len();
    assert!(after > before, "ticker should add at least one fresh sample");

    // The point still carries a numeric live value.
    let point = store.get_point(temp.id).unwrap();
    assert!(matches!(point.cur_value, Some(PointValue::Number(_))));
}
