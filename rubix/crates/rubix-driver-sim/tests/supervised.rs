//! Live supervisor ↔ driver test: the real `Supervisor` spawns the compiled
//! sim binary, the sim attaches to the bus (liveliness token) and publishes
//! `cur`, a second peer observes the sample, and shutdown clears the token.
//! This is the end-to-end spawn path that unit tests (which fail closed before
//! spawn) cannot cover.

use std::time::Duration;

use rubix_driver::{Access, Capability, CapabilitySet, DriverManifest, Identity, Launch};
use rubix_server::supervisor::{liveliness_key, Supervisor};
use serde_json::json;

/// Manifest pointing at the compiled sim binary, granting it publish on the
/// test point and configuring a fast (1s) sample period.
fn sim_manifest(point: &str) -> DriverManifest {
    DriverManifest {
        identity: Identity {
            name: "sim-temp".into(),
            protocol: "sim".into(),
            version: "0.1.0".into(),
            launch: Launch {
                command: env!("CARGO_BIN_EXE_rubix-driver-sim").into(),
                args: vec![],
            },
        },
        point_types: vec![],
        capabilities: CapabilitySet {
            grants: vec![Capability {
                prefix: point.rsplit_once('/').map(|(p, _)| p).unwrap_or(point).into(),
                access: Access::Publish,
            }],
        },
        config: json!({ "point": point, "period_secs": 1, "baseline": 21.0, "amplitude": 2.0 }),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn supervisor_spawns_sim_which_attaches_and_publishes() {
    let point = "sim/site-a/ahu-1/temp";

    // A peer subscribing to the sim's cur, and the session the supervisor uses.
    let client = zenoh::open(zenoh::Config::default()).await.expect("client");
    let sub = client
        .declare_subscriber(format!("{point}/cur"))
        .await
        .expect("subscribe");

    let sup_session = zenoh::open(zenoh::Config::default())
        .await
        .expect("sup session");
    let supervisor = Supervisor::launch(sup_session, vec![sim_manifest(point)]).expect("launch");

    // The sim should declare its liveliness token shortly after spawn.
    let attached = {
        let key = liveliness_key("sim-temp");
        let mut ok = false;
        for _ in 0..50 {
            if let Ok(replies) = client.liveliness().get(&key).await {
                if let Ok(Ok(reply)) =
                    tokio::time::timeout(Duration::from_millis(200), replies.recv_async()).await
                {
                    if reply.result().is_ok() {
                        ok = true;
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        ok
    };
    assert!(attached, "sim did not declare its liveliness token");

    // And it should publish a cur sample within a couple of periods.
    let sample = tokio::time::timeout(Duration::from_secs(5), sub.recv_async())
        .await
        .expect("a cur sample within timeout")
        .expect("sample");
    let value: f64 = serde_json::from_slice(&sample.payload().to_bytes()).expect("decode cur");
    assert!((19.0..=23.0).contains(&value), "unexpected sim value {value}");

    // Shutdown kills the sim; its liveliness token then clears.
    supervisor.shutdown().await;
    let mut cleared = false;
    for _ in 0..50 {
        // The liveliness subscriber sees a Delete sample when the token clears,
        // but a fresh get returning no live reply is the simplest assertion.
        let key = liveliness_key("sim-temp");
        let live = match client.liveliness().get(&key).await {
            Ok(replies) => matches!(
                tokio::time::timeout(Duration::from_millis(200), replies.recv_async()).await,
                Ok(Ok(reply)) if reply.result().is_ok()
            ),
            Err(_) => false,
        };
        if !live {
            cleared = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(cleared, "sim liveliness token did not clear after shutdown");

    // Keep the subscriber alive to the end so the borrow is not dropped early.
    drop(sub);
}
