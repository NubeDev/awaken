//! The navigation tree model (docs/design/page-context-and-nav.md §4): a nested,
//! org-scoped tree where each node assigns a board (with a context payload), a
//! static app route, or is a non-clickable group header. A nav node *mounts* a
//! (possibly shared) board with a context — it is not a scope. The same board can
//! legitimately appear under two nodes with different contexts; that is the fleet
//! story this model exists for.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::CoreError;

/// The closed allow-list of built-in static app pages a `route` node may target
/// (docs/design/page-context-and-nav.md §4). Not free-form: a route node is the
/// access-gate surface for a built-in page, so its value must resolve to a known
/// router entry. Adding a page to the UI router means adding a variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NavRoute {
    Sites,
    Equips,
    Points,
    Dashboards,
    Datasources,
    Rules,
    Boards,
    Sparks,
    Runs,
    Audit,
    Access,
}

impl NavRoute {
    /// The canonical wire token (matches the UI router segment).
    pub fn as_str(self) -> &'static str {
        match self {
            NavRoute::Sites => "sites",
            NavRoute::Equips => "equips",
            NavRoute::Points => "points",
            NavRoute::Dashboards => "dashboards",
            NavRoute::Datasources => "datasources",
            NavRoute::Rules => "rules",
            NavRoute::Boards => "boards",
            NavRoute::Sparks => "sparks",
            NavRoute::Runs => "runs",
            NavRoute::Audit => "audit",
            NavRoute::Access => "access",
        }
    }

    /// Parse the canonical token; an unknown route fails closed (`None`).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "sites" => NavRoute::Sites,
            "equips" => NavRoute::Equips,
            "points" => NavRoute::Points,
            "dashboards" => NavRoute::Dashboards,
            "datasources" => NavRoute::Datasources,
            "rules" => NavRoute::Rules,
            "boards" => NavRoute::Boards,
            "sparks" => NavRoute::Sparks,
            "runs" => NavRoute::Runs,
            "audit" => NavRoute::Audit,
            "access" => NavRoute::Access,
            _ => return None,
        })
    }

    /// Every route, for seeding the default tree on org provision.
    pub const ALL: [NavRoute; 11] = [
        NavRoute::Sites,
        NavRoute::Equips,
        NavRoute::Points,
        NavRoute::Dashboards,
        NavRoute::Datasources,
        NavRoute::Rules,
        NavRoute::Boards,
        NavRoute::Sparks,
        NavRoute::Runs,
        NavRoute::Audit,
        NavRoute::Access,
    ];
}

/// What a nav node opens — a tagged union (docs/design/page-context-and-nav.md
/// §4). A `group` is a non-clickable header; a `dashboard` mounts a reusable
/// board (its id is validated against the caller's org in the handler — a bare FK
/// cannot encode org-safety since `dashboards.id` is a global PK with `org` a
/// separate column); a `route` opens a built-in static page from the closed
/// allow-list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NavTarget {
    /// A header that expands/collapses; not a link.
    Group,
    /// A mount of a reusable dashboard board.
    Dashboard { dashboard_id: Uuid },
    /// A built-in static app page (closed allow-list).
    Route {
        #[schema(value_type = String)]
        route: NavRoute,
    },
}

impl NavTarget {
    /// The dashboard id this target mounts, if any. Used by the org-scoped
    /// validation in the handler and the board-delete sweep.
    pub fn dashboard_id(&self) -> Option<Uuid> {
        match self {
            NavTarget::Dashboard { dashboard_id } => Some(*dashboard_id),
            _ => None,
        }
    }
}

/// A dashboard node's context payload (docs/design/page-context-and-nav.md §1).
/// Exactly `{ values?, tags? }` — not arbitrary top-level keys. `values`
/// overrides variable resolution (`$site` from `values.site`); `tags` merge over
/// the board's own tags. Only meaningful on a `dashboard` target.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NavContext {
    /// Variable-resolution overrides keyed by variable name. The injection
    /// boundary is preserved downstream: every value binds as a SQL parameter via
    /// the variables engine, never string-concatenated.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, serde_json::Value>,
    /// Tag pins merged over the board's own tags for this mount.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
}

impl NavContext {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.tags.is_empty()
    }
}

/// One navigation-tree node. Org-scoped and nestable (`parent_id` self-ref, NULL
/// = root). `context` is only meaningful on a `dashboard` target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NavNode {
    pub id: Uuid,
    /// Owning org namespace (the tenant key). Every read filters by it.
    pub org: String,
    /// Parent node; `None` marks a root node. Nestable arbitrarily deep.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub title: String,
    /// Sibling ordering within a parent.
    pub sort_order: i64,
    pub target: NavTarget,
    /// Context payload (dashboard targets only). Absent / empty for group and
    /// route nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<NavContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}

impl NavNode {
    /// Reject a node whose shape is internally inconsistent: a `context` payload
    /// only makes sense on a `dashboard` target. A group/route node must not carry
    /// one (it would silently never be read). Identity and org-safety are checked
    /// in the handler against the store; this is the pure shape rule.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.title.trim().is_empty() {
            return Err(CoreError::InvalidNavNode("title must be non-empty".into()));
        }
        let has_ctx = self.context.as_ref().is_some_and(|c| !c.is_empty());
        if has_ctx && self.target.dashboard_id().is_none() {
            return Err(CoreError::InvalidNavNode(
                "context is only valid on a dashboard target".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(target: NavTarget, context: Option<NavContext>) -> NavNode {
        NavNode {
            id: Uuid::new_v4(),
            org: "kfc".into(),
            parent_id: None,
            title: "Buildings".into(),
            sort_order: 0,
            target,
            context,
            icon: None,
            accent: None,
        }
    }

    #[test]
    fn route_token_round_trips_and_fails_closed() {
        for r in NavRoute::ALL {
            assert_eq!(NavRoute::parse(r.as_str()), Some(r));
        }
        assert!(NavRoute::parse("secret").is_none());
    }

    #[test]
    fn context_only_valid_on_dashboard_target() {
        let ctx = NavContext {
            values: BTreeMap::from([("site".into(), serde_json::json!("s1"))]),
            tags: BTreeMap::new(),
        };
        // A dashboard target may carry context.
        assert!(node(
            NavTarget::Dashboard { dashboard_id: Uuid::new_v4() },
            Some(ctx.clone())
        )
        .validate()
        .is_ok());
        // A group/route node must not.
        assert!(node(NavTarget::Group, Some(ctx.clone())).validate().is_err());
        assert!(node(
            NavTarget::Route { route: NavRoute::Datasources },
            Some(ctx)
        )
        .validate()
        .is_err());
        // Empty context is fine on any target.
        assert!(node(NavTarget::Group, Some(NavContext::default()))
            .validate()
            .is_ok());
    }

    #[test]
    fn empty_title_rejected() {
        assert!(node(NavTarget::Group, None)
            .validate()
            .is_ok());
        let mut n = node(NavTarget::Group, None);
        n.title = "  ".into();
        assert!(n.validate().is_err());
    }

    #[test]
    fn target_tagged_union_round_trips() {
        let t = NavTarget::Route { route: NavRoute::Audit };
        let json = serde_json::to_value(&t).unwrap();
        assert_eq!(json, serde_json::json!({"kind": "route", "route": "audit"}));
        let back: NavTarget = serde_json::from_value(json).unwrap();
        assert_eq!(back, t);
    }
}
