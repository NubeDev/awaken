//! [`WidgetAccess`] over the store: lets the agent `pin_widget` tool persist a
//! dashboard tile. The store enforces that the owning site exists.
//!
//! When the run is tenant-scoped, pinning is gated to sites within the run's
//! `{org}/{site}`: the owning site is resolved and its `{org}/{slug}` checked
//! against the scope, so a scoped agent cannot pin a tile on another tenant's
//! dashboard.

use async_trait::async_trait;
use chrono::Utc;
use rubix_core::{Widget, WidgetKind};
use rubix_tools::{TenantScope, WidgetAccess};
use uuid::Uuid;

use crate::store::Store;

/// Store-backed widget pinning handed to the `pin_widget` tool. An optional
/// [`TenantScope`] confines pinning to sites in one `{org}/{site}`.
#[derive(Clone)]
pub struct StoreWidgetAccess {
    store: Store,
    scope: Option<TenantScope>,
}

impl StoreWidgetAccess {
    /// Unscoped widget access: any existing site may be pinned to.
    pub fn new(store: Store) -> Self {
        Self { store, scope: None }
    }

    /// Widget access confined to `scope` when present.
    pub fn scoped(store: Store, scope: Option<TenantScope>) -> Self {
        Self { store, scope }
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
        if let Some(scope) = &self.scope {
            // Resolve the owning site and reject a pin outside the run's tenant.
            let site = self.store.get_site(site_id)?;
            if !scope.covers(&format!("{}/{}", site.org, site.slug)) {
                anyhow::bail!(
                    "site `{}/{}` is outside the run's tenant scope `{}`",
                    site.org,
                    site.slug,
                    scope.scope_id()
                );
            }
        }
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
