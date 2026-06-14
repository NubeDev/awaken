//! Navigation-tree rows (docs/design/page-context-and-nav.md §4): org-scoped,
//! nestable nodes that each mount a board (with context), a static route, or a
//! group header. `target`/`context` persist as JSON columns; `title` and other
//! free-form text reach SQL only as bound parameters (the injection boundary).
//! Backend dispatch; SQLite body inline, Postgres in [`super::postgres::nav_nodes`].

use rubix_core::{NavContext, NavNode, NavTarget};
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_of, json_to};
use super::{Result, Store, StoreError};

pub(crate) const NAV_COLS: &str =
    "id, org, parent_id, title, sort_order, target, context, icon, accent";

fn row_nav_node(row: &Row<'_>) -> rusqlite::Result<NavNode> {
    let target_raw: String = row.get(5)?;
    let context_raw: Option<String> = row.get(6)?;
    let target: NavTarget = json_to(&target_raw)?;
    let context: Option<NavContext> = match context_raw {
        Some(s) => Some(json_to(&s)?),
        None => None,
    };
    Ok(NavNode {
        id: row.get(0)?,
        org: row.get(1)?,
        parent_id: row.get(2)?,
        title: row.get(3)?,
        sort_order: row.get(4)?,
        target,
        context,
        icon: row.get(7)?,
        accent: row.get(8)?,
    })
}

impl Store {
    pub fn create_nav_node(&self, node: &NavNode) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.create_nav_node_sqlite(node),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::nav_nodes::create_nav_node(self, node),
        }
    }

    fn create_nav_node_sqlite(&self, node: &NavNode) -> Result<()> {
        let conn = self.sqlite_conn()?;
        // A parent must live in the same org — a node cannot reparent across
        // tenants. NULL parent (root) skips the check.
        if let Some(parent) = node.parent_id {
            Self::require_nav_node_in_org(&conn, parent, &node.org)?;
        }
        let context = node
            .context
            .as_ref()
            .filter(|c| !c.is_empty())
            .map(json_of);
        conn.execute(
            "INSERT INTO nav_nodes (id, org, parent_id, title, sort_order, target, context, \
             icon, accent) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                node.id,
                node.org,
                node.parent_id,
                node.title,
                node.sort_order,
                json_of(&node.target),
                context,
                node.icon,
                node.accent,
            ],
        )?;
        Ok(())
    }

    /// Every nav node in `org`, ordered for tree assembly (`parent_id` then
    /// `sort_order`). The caller filters to nodes the principal holds `view` on.
    pub fn list_nav_nodes(&self, org: &str) -> Result<Vec<NavNode>> {
        match &self.backend {
            Backend::Sqlite(_) => self.list_nav_nodes_sqlite(org),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::nav_nodes::list_nav_nodes(self, org),
        }
    }

    fn list_nav_nodes_sqlite(&self, org: &str) -> Result<Vec<NavNode>> {
        let conn = self.sqlite_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {NAV_COLS} FROM nav_nodes WHERE org = ?1 \
             ORDER BY parent_id IS NOT NULL, parent_id, sort_order, title"
        ))?;
        let rows = stmt.query_map(params![org], row_nav_node)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get_nav_node(&self, id: Uuid) -> Result<NavNode> {
        match &self.backend {
            Backend::Sqlite(_) => self
                .sqlite_conn()?
                .query_row(
                    &format!("SELECT {NAV_COLS} FROM nav_nodes WHERE id = ?1"),
                    params![id],
                    row_nav_node,
                )
                .optional()?
                .ok_or(StoreError::NotFound("nav node")),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::nav_nodes::get_nav_node(self, id),
        }
    }

    /// Patch the mutable fields of a node. `org` is identity and immutable;
    /// `parent_id`/`sort_order` carry reparent + reorder. A `None` field is left
    /// unchanged via a `?N IS NULL` sentinel; `context` is replaced wholesale.
    /// Returns the updated row.
    pub fn update_nav_node(&self, node: &NavNode) -> Result<NavNode> {
        match &self.backend {
            Backend::Sqlite(_) => self.update_nav_node_sqlite(node),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::nav_nodes::update_nav_node(self, node),
        }
    }

    fn update_nav_node_sqlite(&self, node: &NavNode) -> Result<NavNode> {
        let conn = self.sqlite_conn()?;
        if let Some(parent) = node.parent_id {
            if parent == node.id {
                return Err(StoreError::Conflict("nav node cannot parent itself".into()));
            }
            Self::require_nav_node_in_org(&conn, parent, &node.org)?;
        }
        let context = node
            .context
            .as_ref()
            .filter(|c| !c.is_empty())
            .map(json_of);
        let n = conn.execute(
            "UPDATE nav_nodes SET parent_id = ?2, title = ?3, sort_order = ?4, \
             target = ?5, context = ?6, icon = ?7, accent = ?8 \
             WHERE id = ?1 AND org = ?9",
            params![
                node.id,
                node.parent_id,
                node.title,
                node.sort_order,
                json_of(&node.target),
                context,
                node.icon,
                node.accent,
                node.org,
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound("nav node"));
        }
        self.get_nav_node(node.id)
    }

    pub fn delete_nav_node(&self, id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self
                    .sqlite_conn()?
                    .execute("DELETE FROM nav_nodes WHERE id = ?1", params![id])?;
                if n == 0 {
                    return Err(StoreError::NotFound("nav node"));
                }
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::nav_nodes::delete_nav_node(self, id),
        }
    }

    /// Sweep every node that mounts `dashboard_id` back to a `group` target (the
    /// board-delete cascade, docs/design/page-context-and-nav.md §4): losing a
    /// board must not delete the nav node. Called from the dashboard delete path.
    pub fn sweep_nav_dashboard(&self, dashboard_id: Uuid) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => self.sweep_nav_dashboard_sqlite(dashboard_id),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::nav_nodes::sweep_nav_dashboard(self, dashboard_id)
            }
        }
    }

    fn sweep_nav_dashboard_sqlite(&self, dashboard_id: Uuid) -> Result<()> {
        // The target is JSON, so match the mounted id with the JSON1 extractor and
        // rewrite both target and context (context is meaningless on a group).
        let group = json_of(&NavTarget::Group);
        self.sqlite_conn()?.execute(
            "UPDATE nav_nodes SET target = ?1, context = NULL \
             WHERE json_extract(target, '$.dashboard_id') = ?2",
            params![group, dashboard_id.to_string()],
        )?;
        Ok(())
    }

    /// Confirm a nav node exists and belongs to `org` (the same-tenant parent /
    /// node guard). Returns [`StoreError::NotFound`] otherwise.
    pub(crate) fn require_nav_node_in_org(
        conn: &rusqlite::Connection,
        id: Uuid,
        org: &str,
    ) -> Result<()> {
        let found: Option<String> = conn
            .query_row("SELECT org FROM nav_nodes WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()?;
        match found {
            Some(node_org) if node_org == org => Ok(()),
            _ => Err(StoreError::NotFound("nav node")),
        }
    }
}
