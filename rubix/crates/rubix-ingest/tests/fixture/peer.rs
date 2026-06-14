//! A local Zenoh peer pair wired over TCP loopback — the transport the ingest
//! subscribe/persist integration tests publish across.
//!
//! The integration spec opens a local Zenoh peer session and publishes samples to
//! a granted key-space (`rubix/docs/sessions/WS-12.md`). To keep the test
//! self-contained and deterministic — no dependency on the host's multicast
//! fabric — the subscriber binds an explicit loopback listen endpoint and the
//! publisher dials it directly, with multicast scouting disabled on both sides.
//! `wait_linked` lets the publisher hold off until the subscriber peer is
//! actually reachable, so the first `put` is not dropped before the link forms.

// Only the `listen` test binary uses this peer pair; the fixture is compiled into
// every ingest test binary via `#[path]`, so the others see it as unused.
#![allow(dead_code)]

use rubix_ingest::ZenohEndpoint;
use zenoh::Session;

/// The loopback listen endpoint the subscriber binds for `port`.
#[must_use]
pub fn listen_endpoint(port: u16) -> ZenohEndpoint {
    ZenohEndpoint {
        listen: vec![format!("tcp/127.0.0.1:{port}")],
        connect: Vec::new(),
        multicast_scouting: false,
    }
}

/// Open a publisher peer that dials the subscriber listening on `port`.
///
/// Multicast scouting is off, so the publisher reaches the subscriber only via
/// the explicit connect endpoint — making the link deterministic in CI.
pub async fn open_publisher(port: u16) -> Session {
    let endpoint = ZenohEndpoint {
        listen: Vec::new(),
        connect: vec![format!("tcp/127.0.0.1:{port}")],
        multicast_scouting: false,
    };
    let config = endpoint.to_config().expect("publisher config");
    zenoh::open(config).await.expect("open publisher session")
}

/// Wait until `session` reports at least one connected peer, or give up after a
/// bounded number of polls.
///
/// Zenoh forms the loopback link asynchronously; publishing before the link is up
/// would silently drop the sample. Polling the session's peer set is the
/// supported way to know the subscriber is reachable.
pub async fn wait_linked(session: &Session) {
    for _ in 0..200 {
        let peers = session.info().peers_zid().await.count();
        if peers > 0 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("publisher never linked to the subscriber peer");
}

/// Publish `payload` (already-encoded JSON bytes) to `key` on `session`.
pub async fn publish(session: &Session, key: &str, payload: &serde_json::Value) {
    let bytes = serde_json::to_vec(payload).expect("encode payload");
    session.put(key, bytes).await.expect("publish sample");
}
