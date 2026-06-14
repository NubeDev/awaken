//! Publish a point's current value on `{keyexpr}/cur`.

use rubix_core::PointValue;

use super::ZenohBus;

impl ZenohBus {
    /// Publish a current-value sample on the point's `cur` keyexpr. Payload is
    /// the JSON-encoded value, or `null` when the point has no effective value
    /// (fully relinquished). Subscribers (dashboards, reflow boards) receive
    /// live updates. Best-effort: a publish failure is logged, not propagated
    /// to the HTTP caller, since the value is already persisted.
    pub async fn publish_cur(&self, keyexpr_prefix: &str, value: Option<&PointValue>) {
        let key = format!("{keyexpr_prefix}/cur");
        let payload = match serde_json::to_vec(&value) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!(key, error = %e, "encode cur payload");
                return;
            }
        };
        if let Err(e) = self.session().put(&key, payload).await {
            tracing::warn!(key, error = %e, "publish cur");
        }
    }
}
