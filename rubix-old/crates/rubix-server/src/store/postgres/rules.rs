//! Stored-rule rows, Postgres backend. Mirrors [`super::super::rules`]. Rules
//! carry an `org` + optional `site_id`; `IS NOT DISTINCT FROM` does the
//! NULL-aware site comparison the SQLite `IS` does.

use rubix_rules::ParamSchema;
use uuid::Uuid;

use super::super::codec::{json_of, ts_of};
use super::super::rules::{RuleRecord, RULE_COLS};
use super::super::{Result, Store, StoreError};
use super::codec::{json_col, ts_col, uuid_of};

fn rule_of(row: &postgres::Row) -> Result<RuleRecord> {
    let site_id = row
        .get::<_, Option<String>>(2)
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| StoreError::Db(anyhow::anyhow!("bad rule site_id uuid: {e}")))?;
    Ok(RuleRecord {
        id: uuid_of(row, 0)?,
        org: row.get(1),
        site_id,
        name: row.get(3),
        script: row.get(4),
        params: json_col(row, 5)?,
        created_at: ts_col(row, 6)?,
    })
}

pub(crate) fn create_rule(store: &Store, rule: &RuleRecord) -> Result<()> {
    let mut client = store.postgres_conn()?;
    if let Some(site_id) = rule.site_id {
        super::codec::require(&mut *client, "sites", "site", site_id)?;
    }
    let site_id = rule.site_id.map(|s| s.to_string());
    client.execute(
        "INSERT INTO rules (id, org, site_id, name, script, params, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        &[
            &rule.id.to_string(),
            &rule.org,
            &site_id,
            &rule.name,
            &rule.script,
            &json_of(&rule.params),
            &ts_of(&rule.created_at),
        ],
    )?;
    Ok(())
}

pub(crate) fn list_rules(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
) -> Result<Vec<RuleRecord>> {
    let site = site_id.map(|s| s.to_string());
    let rows = store.postgres_conn()?.query(
        &format!(
            "SELECT {RULE_COLS} FROM rules \
             WHERE org = $1 AND ($2::text IS NULL OR site_id IS NULL OR site_id = $2) \
             ORDER BY created_at DESC"
        ),
        &[&org, &site],
    )?;
    rows.iter().map(rule_of).collect()
}

pub(crate) fn load_rule(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    name: &str,
) -> Result<RuleRecord> {
    // Site-scoped rule wins, then the org-level one.
    if let Some(site_id) = site_id {
        if let Some(row) = store.postgres_conn()?.query_opt(
            &format!(
                "SELECT {RULE_COLS} FROM rules WHERE org = $1 AND site_id = $2 AND name = $3"
            ),
            &[&org, &site_id.to_string(), &name],
        )? {
            return rule_of(&row);
        }
    }
    let row = store
        .postgres_conn()?
        .query_opt(
            &format!(
                "SELECT {RULE_COLS} FROM rules WHERE org = $1 AND site_id IS NULL AND name = $2"
            ),
            &[&org, &name],
        )?
        .ok_or(StoreError::NotFound("rule"))?;
    rule_of(&row)
}

pub(crate) fn load_rule_exact(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    name: &str,
) -> Result<RuleRecord> {
    let site = site_id.map(|s| s.to_string());
    let row = store
        .postgres_conn()?
        .query_opt(
            &format!(
                "SELECT {RULE_COLS} FROM rules \
                 WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND name = $3"
            ),
            &[&org, &site, &name],
        )?
        .ok_or(StoreError::NotFound("rule"))?;
    rule_of(&row)
}

pub(crate) fn update_rule(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    name: &str,
    script: &str,
    params: &ParamSchema,
) -> Result<RuleRecord> {
    let site = site_id.map(|s| s.to_string());
    let n = store.postgres_conn()?.execute(
        "UPDATE rules SET script = $4, params = $5 \
         WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND name = $3",
        &[&org, &site, &name, &script, &json_of(params)],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("rule"));
    }
    load_rule_exact(store, org, site_id, name)
}

pub(crate) fn delete_rule(
    store: &Store,
    org: &str,
    site_id: Option<Uuid>,
    name: &str,
) -> Result<()> {
    let site = site_id.map(|s| s.to_string());
    let n = store.postgres_conn()?.execute(
        "DELETE FROM rules WHERE org = $1 AND site_id IS NOT DISTINCT FROM $2 AND name = $3",
        &[&org, &site, &name],
    )?;
    if n == 0 {
        return Err(StoreError::NotFound("rule"));
    }
    Ok(())
}
