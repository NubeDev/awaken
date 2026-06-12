//! Flow integration: a reflow board's `PointAccess` reads and commands real
//! store points through the priority array.

use rubix_core::PointValue;
use rubix_flow::PointAccess;
use rubix_server::flow::StorePointAccess;

use super::harness::TestApp;

#[tokio::test]
async fn store_point_access_reads_writes_and_histories() {
    let (app, store) = TestApp::with_store();
    let site = app.create_site().await;
    let equip = app.create_equip(&site).await;
    let _point = app.create_point(&equip, "cmd", "fan").await;
    let keyexpr = "nube/hq/ahu-3/fan";

    let access = StorePointAccess::new(store);

    // No command yet → no effective value.
    assert_eq!(access.read_point(keyexpr).unwrap(), None);

    // Command priority 8 → becomes the effective value, readable back.
    let effective = access
        .write_point(keyexpr, 8, PointValue::Bool(true))
        .unwrap();
    assert_eq!(effective, Some(PointValue::Bool(true)));
    assert_eq!(
        access.read_point(keyexpr).unwrap(),
        Some(PointValue::Bool(true))
    );

    // The command landed in history.
    let his = access.query_his(keyexpr, 10).unwrap();
    assert_eq!(his.len(), 1);
    assert_eq!(his[0].value, PointValue::Bool(true));
}

#[tokio::test]
async fn unknown_keyexpr_is_an_error() {
    let (_app, store) = TestApp::with_store();
    let access = StorePointAccess::new(store);
    assert!(access.read_point("no/such/point/here").is_err());
}
