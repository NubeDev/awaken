//! Default-tree seeding on org provision (docs/design/page-context-and-nav.md
//! §6). A fresh org gets one root group plus a `route` node for every built-in
//! static page, so gating a static page becomes opt-in *tightening* rather than a
//! lockout — nothing silently disappears the first time the nav tree gates a
//! built-in page. Default visibility is the Layer-1 org-scope read (`nav_node`
//! grants only ever ADD or narrow access), so no explicit grant rows are seeded.

use rubix_core::{NavNode, NavRoute, NavTarget};
use uuid::Uuid;

use crate::store::Store;
use crate::store::StoreError;

/// Seed the default nav tree for a newly provisioned `org`. Idempotent guard: the
/// caller only invokes this on first provision, but the function itself simply
/// inserts the standard set — a "Pages" root group with each route under it.
pub(crate) fn seed_default_tree(store: &Store, org: &str) -> Result<(), StoreError> {
    let root = NavNode {
        id: Uuid::new_v4(),
        org: org.to_string(),
        parent_id: None,
        title: "Pages".into(),
        sort_order: 0,
        target: NavTarget::Group,
        context: None,
        icon: None,
        accent: None,
    };
    store.create_nav_node(&root)?;
    for (i, route) in NavRoute::ALL.iter().enumerate() {
        let node = NavNode {
            id: Uuid::new_v4(),
            org: org.to_string(),
            parent_id: Some(root.id),
            title: title_for(*route).to_string(),
            sort_order: i as i64,
            target: NavTarget::Route { route: *route },
            context: None,
            icon: None,
            accent: None,
        };
        store.create_nav_node(&node)?;
    }
    Ok(())
}

/// A human-facing default title for a route node (the operator can rename later).
fn title_for(route: NavRoute) -> &'static str {
    match route {
        NavRoute::Sites => "Sites",
        NavRoute::Equips => "Equipment",
        NavRoute::Points => "Points",
        NavRoute::Dashboards => "Dashboards",
        NavRoute::Datasources => "Datasources",
        NavRoute::Rules => "Rules",
        NavRoute::Boards => "Boards",
        NavRoute::Sparks => "Sparks",
        NavRoute::Runs => "Runs",
        NavRoute::Audit => "Audit",
        NavRoute::Access => "Access",
    }
}
