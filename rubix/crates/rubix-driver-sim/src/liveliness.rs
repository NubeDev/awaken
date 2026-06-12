//! Declare the driver's liveliness token so the supervisor's `await_attach`
//! sees the driver actually on the bus (not merely a started process). The
//! token clears when the returned handle drops or the session closes — which
//! is how the supervisor reaps a crashed or stopped driver.

use zenoh::Session;

/// Liveliness-token keyexpr for a driver named `name`. Must match the
/// supervisor's `liveliness_key` (`rubix/liveliness/driver/{name}`).
pub fn liveliness_key(name: &str) -> String {
    format!("rubix/liveliness/driver/{name}")
}

/// Declare the liveliness token. The returned token is held for the process
/// lifetime; dropping it (on shutdown/crash) clears the token on the mesh.
pub async fn declare(session: &Session, name: &str) -> anyhow::Result<zenoh::liveliness::LivelinessToken> {
    let key = liveliness_key(name);
    session
        .liveliness()
        .declare_token(&key)
        .await
        .map_err(|e| anyhow::anyhow!("declare liveliness token {key}: {e}"))
}
