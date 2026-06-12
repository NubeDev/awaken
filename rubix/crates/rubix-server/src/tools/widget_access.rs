//! [`WidgetAccess`] over the store: lets the agent `pin_widget` tool persist a
//! dashboard tile. The store enforces that the owning site exists.

use async_trait::async_trait;
use chrono::Utc;
use rubix_core::{Widget, WidgetKind};
use rubix_tools::WidgetAccess;
use uuid::Uuid;

use crate::store::Store;

/// Store-backed widget pinning handed to the `pin_widget` tool.
#[derive(Clone)]
pub struct StoreWidgetAccess {
    store: Store,
}

impl StoreWidgetAccess {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

#[async_trait]
impl WidgetAccess for StoreWidgetAccess {
    async fn pin_widget(
        &self,
        site_id: Uuid,
        kind: &str,
        title: &str,
        target: &str,
    ) -> anyhow::Result<Uuid> {
        let kind: WidgetKind = serde_json::from_str(&format!("\"{kind}\""))
            .map_err(|_| anyhow::anyhow!("unknown widget kind: {kind}"))?;
        let widget = Widget {
            id: Uuid::new_v4(),
            site_id,
            kind,
            title: title.to_string(),
            target: target.to_string(),
            created_at: Utc::now(),
        };
        self.store.create_widget(&widget)?;
        Ok(widget.id)
    }
}
