//! Navigation-tree rows, Postgres backend. Mirrors [`super::super::nav_nodes`].

use rubix_core::{NavContext, NavNode, NavTarget};
use uuid::Uuid;

use super::super::codec::{json_of, json_to};
use super::super::nav_nodes::NAV_COLS;
use super::super::{Result, Store, StoreError};
use super::codec::uuid_of;

fn nav_node_of(row: &postgres::Row) -> Result<NavNode> {
    let parent_id = row
        .get::<_, Option<String>>(2)
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| StoreError::Db(anyhow::anyhow!("bad nav parent_id uuid: {e}")))?;
    let target_raw: String = row.get(5);
    let context_raw: Option<String> = row.get(6);
    let target: NavTarget =
        json_to(&target_raw).map_err(|e| StoreError::Db(anyhow::anyhow!("bad nav target: {e}")))?;
    let context: Option<NavContext> = match context_raw {
        Some(s) => Some(
            json_to(&s).map_err(|e| StoreError::Db(anyhow::anyhow!("bad nav context: {e}")))?,
        ),
        None => None,
    };
    Ok(NavNode {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        parent_id,
        title: row.get(3),
        sort_order: row.get(4),
        target,
        context,
        icon: row.get(7),
        accent: row.get(8),
    })
}

fn encoded_context(node: &NavNode) -> Option<String> {
    node.context
        .as_ref()
        .filter(|c| !c.is_empty())
        .map(json_of)
}

pub(crate) fn create_nav_node(store: &Store, node: &NavNode) -> Result<()> {
    let mut client = store.postgres_conn()?;
    if let Some(parent) = node.parent_id {
        require_in_org(&mut client, parent, &node.org)?;
    }
    let parent = node.parent_id.map(|p| p.to_string());
    client.execute(
        "INSERT INTO nav_nodes (id, org, parent_id, title, sort_order, target, context, \
         icon, accent) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &[
            &node.id.to_string(),
            &node.org,
            &parent,
            &node.title,
            &node.sort_order,
            &json_of(&node.target),
            &encoded_context(node),
            &node.icon,
            &node.accent,
        ],
    )?;
    Ok(())
}

pub(crate) fn list_nav_nodes(store: &Store, org: &str) -> Result<Vec<NavNode>> {
    let mut client = store.postgres_conn()?;
    let sql = format!(
        "SELECT {NAV_COLS} FROM nav_nodes WHERE org = $1 \
         ORDER BY parent_id IS NOT NULL, parent_id, sort_order, title"
    );
    let rows = client.query(sql.as_str(), &[&org])?;
    rows.iter().map(nav_node_of).collect()
}

pub(crate) fn get_nav_node(store: &Store, id: Uuid) -> Result<NavNode> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {NAV_COLS} FROM nav_nodes WHERE id = $1");
    let row = client
        .query_opt(sql.as_str(), &[&id.to_string()])?
        .ok_or(StoreError::NotFound("nav node"))?;
    nav_node_of(&row)
}

pub(crate) fn update_nav_node(store: &Store, node: &NavNode) -> Result<NavNode> {
    let mut client = store.postgres_conn()?;
    if let Some(parent) = node.parent_id {
        if parent == node.id {
            return Err(StoreError::Conflict("nav node cannot parent itself".into()));
        }
        require_in_org(&mut client, parent, &node.org)?;
    }
    let parent = node.parent_id.map(|p| p.to_string());
    let sql = format!(
        "UPDATE nav_nodes SET parent_id = $2, title = $3, sort_order = $4, \
         target = $5, context = $6, icon = $7, accent = $8 \
         WHERE id = $1 AND org = $9 RETURNING {NAV_COLS}"
    );
    let row = client
        .query_opt(
            sql.as_str(),
            &[
                &node.id.to_string(),
                &parent,
                &node.title,
                &node.sort_order,
                &json_of(&node.target),
                &encoded_context(node),
                &node.icon,
                &node.accent,
                &node.org,
            ],
        )?
        .ok_or(StoreError::NotFound("nav node"))?;
    nav_node_of(&row)
}

pub(crate) fn delete_nav_node(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM nav_nodes WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("nav node"));
    }
    Ok(())
}

pub(crate) fn sweep_nav_dashboard(store: &Store, dashboard_id: Uuid) -> Result<()> {
    let group = json_of(&NavTarget::Group);
    store.postgres_conn()?.execute(
        "UPDATE nav_nodes SET target = $1, context = NULL \
         WHERE (target::json ->> 'dashboard_id') = $2",
        &[&group, &dashboard_id.to_string()],
    )?;
    Ok(())
}

fn require_in_org(client: &mut postgres::Client, id: Uuid, org: &str) -> Result<()> {
    let row = client.query_opt(
        "SELECT org FROM nav_nodes WHERE id = $1",
        &[&id.to_string()],
    )?;
    match row {
        Some(r) if r.get::<_, String>(0) == org => Ok(()),
        _ => Err(StoreError::NotFound("nav node")),
    }
}
