//! Driver health via zenoh liveliness tokens. A driver declares a token at
//! [`liveliness_key`]; the supervisor queries it to confirm the driver has
//! actually attached to the bus after spawn, not merely started a process.

use std::time::Duration;

use futures::StreamExt;
use zenoh::Session;

/// Liveliness-token keyexpr a driver named `name` declares while alive.
pub fn liveliness_key(name: &str) -> String {
    format!("rubix/liveliness/driver/{name}")
}

/// True if the driver's liveliness token is currently present on the mesh.
/// Used after spawn to confirm bus attachment, and at boot to detect stale
/// tokens left by a crashed predecessor.
pub async fn is_alive(session: &Session, name: &str) -> bool {
    let key = liveliness_key(name);
    let Ok(replies) = session.liveliness().get(&key).await else {
        return false;
    };
    // A single present token is enough; bounded so a silent mesh can't hang us.
    match tokio::time::timeout(Duration::from_secs(2), replies.recv_async()).await {
        Ok(Ok(reply)) => reply.result().is_ok(),
        _ => false,
    }
}

/// Wait up to `timeout` for the driver to declare its liveliness token after
/// spawn. Returns `true` on attachment, `false` on timeout.
pub async fn await_attach(session: &Session, name: &str, timeout: Duration) -> bool {
    let key = liveliness_key(name);
    let Ok(sub) = session.liveliness().declare_subscriber(&key).await else {
        return false;
    };
    if is_alive(session, name).await {
        return true;
    }
    let mut stream = sub.stream();
    matches!(
        tokio::time::timeout(timeout, stream.next()).await,
        Ok(Some(sample)) if sample.kind() == zenoh::sample::SampleKind::Put
    )
}
