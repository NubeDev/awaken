//! Boot-time reaping of stale driver liveliness. A driver process orphaned by
//! a previous supervisor crash still holds its liveliness token; spawning a
//! second instance would double-drive the field bus. Before first spawn we
//! wait for any pre-existing token to clear.

use std::time::Duration;

use zenoh::Session;

use super::health::is_alive;

/// Wait until driver `name` shows no liveliness token, up to `timeout`. Returns
/// `true` once clear, `false` if a stale token persists past the deadline (the
/// caller should refuse to spawn a duplicate and surface the orphan).
pub async fn await_clear(session: &Session, name: &str, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if !is_alive(session, name).await {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            tracing::warn!(
                driver = name,
                "stale liveliness token present at boot; orphaned driver process suspected"
            );
            return false;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
