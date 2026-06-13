//! Dashboard rows, Postgres backend. Mirrors [`super::super::dashboards`].

use rubix_core::Dashboard;
use uuid::Uuid;

use super::super::codec::ts_of;
use super::super::dashboards::DASHBOARD_COLS;
use super::super::{Result, Store, StoreError};
use super::codec::{require, ts_col, uuid_of};

fn dashboard_of(row: &postgres::Row) -> Result<Dashboard> {
    let site_id = row
        .get::<_, Option<String>>(2)
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| StoreError::Db(anyhow::anyhow!("bad dashboard site_id uuid: {e}")))?;
    Ok(Dashboard {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        site_id,
        slug: row.get(3),
        title: row.get(4),
        created_at: ts_col(row, 5)?,
    })
}

pub(crate) fn create_dashboard(store: &Store, dashboard: &Dashboard) -> Result<()> {
    let mut client = store.postgres_conn()?;
    if let Some(site_id) = dashboard.site_id {
        require(&mut *client, "sites", "site", site_id)?;
    }
    let site_id = dashboard.site_id.map(|s| s.to_string());
    client.execute(
        "INSERT INTO dashboards (id, org, site_id, slug, title, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &dashboard.id.to_string(),
            &dashboard.org,
            &site_id,
            &dashboard.slug,
            &dashboard.title,
            &ts_of(&dashboard.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_dashboards(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> Result<Vec<Dashboard>> {
    let mut client = store.postgres_conn()?;
    let site = site_id.map(|s| s.to_string());
    let sql = format!(
        "SELECT {DASHBOARD_COLS} FROM dashboards \
         WHERE org = $1 AND ($2::text IS NULL OR site_id = $2) \
         ORDER BY site_id IS NOT NULL, slug"
    );
    let rows = client.query(sql.as_str(), &[&org, &site])?;
    rows.iter().map(dashboard_of).collect()
}

pub(crate) fn get_dashboard(store: &Store, id: Uuid) -> Result<Dashboard> {
    let mut client = store.postgres_conn()?;
    let sql = format!("SELECT {DASHBOARD_COLS} FROM dashboards WHERE id = $1");
    let row = client
        .query_opt(sql.as_str(), &[&id.to_string()])?
        .ok_or(StoreError::NotFound("dashboard"))?;
    dashboard_of(&row)
}

pub(crate) fn update_dashboard(store: &Store, id: Uuid, title: Option<&str>) -> Result<Dashboard> {
    let mut client = store.postgres_conn()?;
    let row = client
        .query_opt(
            &format!(
                "UPDATE dashboards SET title = COALESCE($2, title) \
                 WHERE id = $1 RETURNING {DASHBOARD_COLS}"
            ),
            &[&id.to_string(), &title],
        )?
        .ok_or(StoreError::NotFound("dashboard"))?;
    dashboard_of(&row)
}

pub(crate) fn default_dashboard_for_site(store: &Store, site: &rubix_core::Site) -> Result<Uuid> {
    let mut client = store.postgres_conn()?;
    if let Some(row) = client.query_opt(
        "SELECT id FROM dashboards WHERE site_id = $1 AND slug = 'default'",
        &[&site.id.to_string()],
    )? {
        return uuid_of(&row, 0);
    }
    drop(client);
    let dashboard = Dashboard {
        id: Uuid::new_v4(),
        org: site.org.clone(),
        site_id: Some(site.id),
        slug: "default".into(),
        title: "Default".into(),
        created_at: chrono::Utc::now(),
    };
    create_dashboard(store, &dashboard)?;
    Ok(dashboard.id)
}

pub(crate) fn delete_dashboard(store: &Store, id: Uuid) -> Result<()> {
    let n = store
        .postgres_conn()?
        .execute("DELETE FROM dashboards WHERE id = $1", &[&id.to_string()])?;
    if n == 0 {
        return Err(StoreError::NotFound("dashboard"));
    }
    Ok(())
}
