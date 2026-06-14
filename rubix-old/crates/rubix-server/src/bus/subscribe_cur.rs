//! Subscribe to `**/cur` publications from drivers and land each sample in the
//! store as a `cur_value` update + history row.
//!
//! The subscriber is declared with `allowed_origin(Locality::Remote)` so it
//! receives only publications from *other* sessions (the protocol drivers, each
//! of which opens its own peer session). The server's own `publish_cur` echoes
//! (on HTTP write / relinquish / ingest) go out on this same session, so
//! `Remote` filtering keeps the server from re-ingesting its own output — no
//! feedback loop, and a command's effective value is never mistaken for an
//! inbound sensor sample.

use chrono::Utc;
use futures::StreamExt;
use rubix_core::{HisSample, PointValue};
use zenoh::sample::{Locality, Sample};

use super::ZenohBus;

impl ZenohBus {
    /// Declare a `**/cur` subscriber (remote-origin only) and spawn a drain loop
    /// that resolves each publication to a stored point and calls
    /// [`crate::store::Store::ingest_cur`]. The loop runs detached for the
    /// lifetime of the bus.
    pub async fn subscribe_cur(&self) -> anyhow::Result<()> {
        let bus = self.clone();
        let subscriber = self
            .session()
            .declare_subscriber("**/cur")
            .allowed_origin(Locality::Remote)
            .await
            .map_err(|e| anyhow::anyhow!("declare cur subscriber: {e}"))?;
        tokio::spawn(async move {
            let mut stream = subscriber.stream();
            while let Some(sample) = stream.next().await {
                bus.handle_cur(sample).await;
            }
        });
        Ok(())
    }

    async fn handle_cur(&self, sample: Sample) {
        let key = sample.key_expr().as_str().to_string();
        // Key is `{prefix}/cur`; strip the trailing `/cur` to get the keyexpr
        // prefix that `point_by_keyexpr` resolves.
        let Some(prefix) = key.strip_suffix("/cur") else {
            return;
        };
        let value: PointValue = match serde_json::from_slice(&sample.payload().to_bytes()) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(key, error = %e, "cur payload not a PointValue; skipping");
                return;
            }
        };
        let his_sample = HisSample {
            ts: Utc::now(),
            value,
        };
        let prefix = prefix.to_string();
        let store = self.store.clone();
        let key_for_log = key.clone();
        let result = tokio::task::spawn_blocking(move || {
            let id = store.point_by_keyexpr(&prefix)?;
            store.ingest_cur(id, &his_sample)
        })
        .await;
        match result {
            Ok(Ok(_point)) => {}
            // Unprovisioned points are expected on a live mesh (a driver may
            // publish before its point exists); log at debug, not warn.
            Ok(Err(e)) => {
                tracing::debug!(key = key_for_log, error = %e, "cur sample dropped: point not found or store error");
            }
            Err(e) => {
                tracing::warn!(key = key_for_log, error = %e, "cur ingest task panicked");
            }
        }
    }
}
