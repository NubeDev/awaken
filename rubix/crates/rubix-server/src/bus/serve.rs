//! Declare the `write` and `his` queryables that resolve keyexprs to store
//! calls. Each runs as a detached task draining its query stream.

use chrono::Utc;
use futures::StreamExt;
use rubix_core::PointValue;
use rubix_driver::{Access, Capability};
use zenoh::query::Query;

use super::ZenohBus;

/// A write command delivered over zenoh: value plus optional priority slot.
#[derive(serde::Deserialize)]
struct WriteCommand {
    value: PointValue,
    #[serde(default = "default_priority")]
    priority: u8,
}

fn default_priority() -> u8 {
    16
}

impl ZenohBus {
    /// Declare `**/write` and `**/his/**` queryables and spawn drain loops.
    /// Returns after the queryables are registered; the loops run detached.
    pub async fn serve(&self) -> anyhow::Result<()> {
        self.serve_write().await?;
        self.serve_his().await?;
        Ok(())
    }

    async fn serve_write(&self) -> anyhow::Result<()> {
        let bus = self.clone();
        let queryable = self
            .session()
            .declare_queryable("**/write")
            .await
            .map_err(|e| anyhow::anyhow!("declare write queryable: {e}"))?;
        tokio::spawn(async move {
            let mut stream = queryable.stream();
            while let Some(query) = stream.next().await {
                bus.handle_write(query).await;
            }
        });
        Ok(())
    }

    async fn serve_his(&self) -> anyhow::Result<()> {
        let bus = self.clone();
        let queryable = self
            .session()
            .declare_queryable("**/his/**")
            .await
            .map_err(|e| anyhow::anyhow!("declare his queryable: {e}"))?;
        tokio::spawn(async move {
            let mut stream = queryable.stream();
            while let Some(query) = stream.next().await {
                bus.handle_his(query).await;
            }
        });
        Ok(())
    }

    /// Resolve `**/write`, command the priority array, reply with the updated
    /// point, and republish the new effective value on `{prefix}/cur` so a
    /// bus-driven command is observable to `cur` subscribers — just like an
    /// HTTP write.
    async fn handle_write(&self, query: Query) {
        let key = query.key_expr().as_str().to_string();
        let Some(prefix) = prefix_before(&key, "write") else {
            return;
        };
        // In a multi-node mesh, every node's wildcard queryable sees every
        // query. Answer only for keys under a site this node owns; stay silent
        // otherwise so the owning node's reply is the only one — no "not found"
        // noise from foreign nodes.
        if !self.owns(prefix).await {
            return;
        }
        let Some(payload) = query.payload() else {
            let _ = query.reply_err("write requires a value payload").await;
            return;
        };
        let cmd: WriteCommand = match serde_json::from_slice(&payload.to_bytes()) {
            Ok(c) => c,
            Err(e) => {
                let _ = query.reply_err(format!("bad write payload: {e}")).await;
                return;
            }
        };
        let prefix = prefix.to_string();
        let store = self.store.clone();
        let task_prefix = prefix.clone();
        let result = tokio::task::spawn_blocking(move || {
            let id = store.point_by_keyexpr(&task_prefix)?;
            store.command_point(id, cmd.priority, Some(cmd.value), Utc::now())
        })
        .await;
        match result {
            Ok(Ok(point)) => {
                self.publish_cur(&prefix, point.cur_value.as_ref()).await;
                if let Ok(body) = serde_json::to_vec(&point) {
                    let _ = query.reply(query.key_expr().clone(), body).await;
                }
            }
            Ok(Err(e)) => {
                let _ = query.reply_err(e.to_string()).await;
            }
            Err(e) => {
                let _ = query.reply_err(format!("write task: {e}")).await;
            }
        }
    }

    async fn handle_his(&self, query: Query) {
        let key = query.key_expr().as_str().to_string();
        // Key is `{prefix}/his/**`; the prefix is everything before `/his`.
        let Some((prefix, _)) = key.split_once("/his") else {
            return;
        };
        if !self.owns(prefix).await {
            return;
        }
        let prefix = prefix.to_string();
        let store = self.store.clone();
        let result = tokio::task::spawn_blocking(move || {
            let id = store.point_by_keyexpr(&prefix)?;
            store.his_query(id, None, None, 1000)
        })
        .await;
        match result {
            Ok(Ok(samples)) => {
                if let Ok(body) = serde_json::to_vec(&samples) {
                    let _ = query.reply(query.key_expr().clone(), body).await;
                }
            }
            Ok(Err(e)) => {
                let _ = query.reply_err(e.to_string()).await;
            }
            Err(e) => {
                let _ = query.reply_err(format!("his task: {e}")).await;
            }
        }
    }
}

impl ZenohBus {
    /// True if `point_prefix` (`{org}/{site}/{equip}/{point}`) falls under a
    /// site this node owns. Reuses [`Capability::covers`] for correct path-
    /// boundary matching (so `nube/hq` does not spuriously cover `nube/hq2`).
    /// Ownership is read live from the store, so sites provisioned after boot
    /// are covered without re-declaring queryables.
    async fn owns(&self, point_prefix: &str) -> bool {
        let store = self.store.clone();
        let prefixes = match tokio::task::spawn_blocking(move || store.owned_site_prefixes()).await
        {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "owned-prefix lookup failed; declining query");
                return false;
            }
            Err(e) => {
                tracing::warn!(error = %e, "owned-prefix task failed; declining query");
                return false;
            }
        };
        prefixes.iter().any(|site| {
            Capability {
                prefix: site.clone(),
                access: Access::All,
            }
            .covers(point_prefix)
        })
    }
}

/// Strip a trailing `/segment` from a keyexpr, returning the prefix.
fn prefix_before<'a>(key: &'a str, suffix: &str) -> Option<&'a str> {
    key.strip_suffix(suffix)?.strip_suffix('/')
}
